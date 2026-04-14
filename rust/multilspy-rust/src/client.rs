use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use dashmap::DashMap;
use fluent_uri::Uri;

use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use super::server::RustAnalyzerServer;
use multilspy_protocol::protocol::common::*;
use multilspy_protocol::protocol::requests::*;
use multilspy_protocol::protocol::responses::*;

fn uri_to_file_path(uri: &str) -> anyhow::Result<PathBuf> {
    let parsed = Uri::parse(uri).map_err(|e| anyhow::anyhow!("invalid URI '{}': {}", uri, e))?;
    Ok(PathBuf::from(parsed.path().as_str()))
}

#[allow(dead_code)]
#[derive(Debug)]
struct LSPFileBuffer {
    uri: String,
    contents: String,
    version: i32,
    language_id: String,
    ref_count: usize,
}

#[derive(Clone)]
pub struct LSPClient {
    server: Arc<RustAnalyzerServer>,
    open_file_buffers: Arc<DashMap<String, LSPFileBuffer>>,
}

impl LSPClient {
    pub async fn new(config: RustAnalyzerConfig) -> anyhow::Result<Self> {
        Ok(Self {
            server: RustAnalyzerServer::start_server(config).await?,
            open_file_buffers: Arc::new(DashMap::new()),
        })
    }

    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.server.shutdown().await?;
        Ok(())
    }

    pub async fn open_file(&self, uri: &str) -> anyhow::Result<()> {
        if let Some(mut entry) = self.open_file_buffers.get_mut(uri) {
            if entry.ref_count == 0 {
                return Err(anyhow::anyhow!(
                    "invalid open file buffer state for {}: zero ref_count",
                    uri
                ));
            }
            entry.ref_count += 1;
            return Ok(());
        }

        let file_path = uri_to_file_path(uri)?;
        let contents = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| anyhow::anyhow!("failed to read file {}: {}", file_path.display(), e))?;

        let language_id = "rust".to_string();
        let version = 0;

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.to_string(),
                language_id: language_id.clone(),
                version,
                text: contents.clone(),
            },
        };

        self.server
            .send_notification("textDocument/didOpen".to_string(), Some(params))
            .await?;

        self.open_file_buffers.insert(
            uri.to_string(),
            LSPFileBuffer {
                uri: uri.to_string(),
                contents,
                version,
                language_id,
                ref_count: 1,
            },
        );

        Ok(())
    }

    pub async fn close_file(&self, uri: &str) -> anyhow::Result<()> {
        let should_close = {
            let mut entry = self
                .open_file_buffers
                .get_mut(uri)
                .ok_or_else(|| anyhow::anyhow!("file not open: {}", uri))?;
            if entry.ref_count == 0 {
                return Err(anyhow::anyhow!(
                    "invalid open file buffer state for {}: zero ref_count",
                    uri
                ));
            }
            entry.ref_count -= 1;
            entry.ref_count == 0
        };

        if should_close {
            let params = DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: uri.to_string(),
                },
            };

            if let Err(err) = self
                .server
                .send_notification("textDocument/didClose".to_string(), Some(params))
                .await
            {
                if let Some(mut entry) = self.open_file_buffers.get_mut(uri) {
                    entry.ref_count = 1;
                }

                return Err(anyhow::Error::from(err))
                    .context(format!("failed to send didClose notification for {}", uri));
            }

            self.open_file_buffers.remove(uri);
        }

        Ok(())
    }

    pub fn get_open_file_text(&self, uri: &str) -> anyhow::Result<String> {
        let entry = self
            .open_file_buffers
            .get(uri)
            .ok_or_else(|| anyhow::anyhow!("file not open: {}", uri))?;
        Ok(entry.contents.clone())
    }

    async fn with_open_file<T, F, Fut>(&self, uri: &str, op: F) -> anyhow::Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        self.open_file(uri).await?;

        let result = op().await;
        let close_result = self.close_file(uri).await;

        match (result, close_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(err), Ok(())) => Err(err),
            (Ok(_), Err(close_err)) => {
                Err(close_err.context(format!("failed to close file after request: {}", uri)))
            }
            (Err(err), Err(close_err)) => Err(err.context(format!(
                "request failed and closing file also failed for {}: {}",
                uri, close_err
            ))),
        }
    }

    async fn send_text_document_request<P, R>(
        &self,
        uri: &str,
        method: &str,
        params: P,
    ) -> anyhow::Result<R>
    where
        P: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let method = method.to_string();
        self.with_open_file(uri, move || async move {
            let result = self
                .server
                .send_request(method, Some(params))
                .await?
                .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
            Ok(serde_json::from_value(result)?)
        })
        .await
    }

    async fn server_capabilities(&self) -> anyhow::Result<ServerCapabilities> {
        self.server.server_capabilities().await.ok_or_else(|| {
            anyhow::anyhow!("server capabilities are unavailable before initialization")
        })
    }

    pub async fn definition(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<DefinitionResponse> {
        let params = DefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
        };
        self.send_text_document_request(&uri, "textDocument/definition", params)
            .await
    }

    pub async fn type_definition(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<TypeDefinitionResponse> {
        let params = TypeDefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
        };
        self.send_text_document_request(&uri, "textDocument/typeDefinition", params)
            .await
    }

    pub async fn references(
        &self,
        uri: String,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> anyhow::Result<ReferencesResponse> {
        let params = ReferencesParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
            context: ReferenceContext {
                include_declaration,
            },
        };
        self.send_text_document_request(&uri, "textDocument/references", params)
            .await
    }

    pub async fn document_symbols(
        &self,
        uri: String,
    ) -> Result<DocumentSymbolResponse, ServerError> {
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        };
        self.send_text_document_request(&uri, "textDocument/documentSymbol", params)
            .await
            .map_err(ServerError::from)
    }

    pub async fn implementation(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<ImplementationResponse> {
        let params = ImplementationParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
        };
        self.send_text_document_request(&uri, "textDocument/implementation", params)
            .await
    }

    pub async fn workspace_symbols(
        &self,
        query: String,
    ) -> anyhow::Result<WorkspaceSymbolResponse> {
        let query = query.trim();
        if query.is_empty() {
            anyhow::bail!("workspace/symbol query must not be empty or whitespace-only");
        }

        let capabilities = self.server_capabilities().await?;
        if !capabilities.supports_workspace_symbol() {
            anyhow::bail!(
                "server does not advertise support for workspace/symbol in initialize capabilities"
            );
        }

        let params = WorkspaceSymbolParams {
            query: query.to_string(),
        };
        let result = self
            .server
            .send_request("workspace/symbol".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;

        serde_json::from_value(result).map_err(|e| {
            anyhow::anyhow!("failed to deserialize for workspace symbols resp, err: {e}")
        })
    }

    pub async fn workspace_symbol_resolve(
        &self,
        symbol: WorkspaceSymbol,
    ) -> anyhow::Result<WorkspaceSymbolResolveResponse> {
        let capabilities = self.server_capabilities().await?;
        if !capabilities.supports_workspace_symbol() {
            anyhow::bail!(
                "server does not advertise support for workspace/symbol in initialize capabilities"
            );
        }
        if !capabilities.supports_workspace_symbol_resolve() {
            anyhow::bail!(
                "server does not advertise support for workspaceSymbol/resolve in initialize capabilities"
            );
        }

        let result = self
            .server
            .send_request("workspaceSymbol/resolve".to_string(), Some(symbol))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;

        serde_json::from_value(result).map_err(|e| {
            anyhow::anyhow!("failed to deserialize for workspace symbol resolve resp, err: {e}")
        })
    }

    pub async fn prepare_call_hierarchy(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<CallHierarchyPrepareResponse> {
        let params = CallHierarchyPrepareParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position { line, character },
            },
        };
        self.send_text_document_request(&uri, "textDocument/prepareCallHierarchy", params)
            .await
    }

    pub async fn incoming_calls(
        &self,
        item: CallHierarchyItem,
    ) -> anyhow::Result<CallHierarchyIncomingCallsResponse> {
        let params = CallHierarchyIncomingCallsParams { item };

        let result = self
            .server
            .send_request("callHierarchy/incomingCalls".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let calls = serde_json::from_value(result)?;

        Ok(calls)
    }

    pub async fn outgoing_calls(
        &self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyOutgoingCallsResponse, ServerError> {
        let params = CallHierarchyOutgoingCallsParams { item };

        let result = self
            .server
            .send_request("callHierarchy/outgoingCalls".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let calls = serde_json::from_value(result)?;

        Ok(calls)
    }
}

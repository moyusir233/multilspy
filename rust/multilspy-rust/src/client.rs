use std::sync::Arc;

use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use super::server::RustAnalyzerServer;
use multilspy_protocol::protocol::common::*;
use multilspy_protocol::protocol::requests::*;
use multilspy_protocol::protocol::responses::*;

#[derive(Clone)]
pub struct LSPClient {
    server: Arc<RustAnalyzerServer>,
}

impl LSPClient {
    pub async fn new(config: RustAnalyzerConfig) -> anyhow::Result<Self> {
        Ok(Self {
            server: RustAnalyzerServer::start_server(config).await?,
        })
    }

    pub async fn shutdown(self) -> anyhow::Result<()> {
        self.server.shutdown().await?;
        Ok(())
    }

    pub async fn definition(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<DefinitionResponse> {
        let params = DefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self
            .server
            .send_request("textDocument/definition".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let definitions = serde_json::from_value(result)?;

        Ok(definitions)
    }

    pub async fn type_definition(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<TypeDefinitionResponse> {
        let params = TypeDefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self
            .server
            .send_request("textDocument/typeDefinition".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let definitions = serde_json::from_value(result)?;

        Ok(definitions)
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
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            context: ReferenceContext {
                include_declaration,
            },
        };

        let result = self
            .server
            .send_request("textDocument/references".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let references = serde_json::from_value(result)?;

        Ok(references)
    }

    pub async fn document_symbols(
        &self,
        uri: String,
    ) -> Result<DocumentSymbolResponse, ServerError> {
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
        };

        let result = self
            .server
            .send_request("textDocument/documentSymbol".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let symbols = serde_json::from_value(result)?;

        Ok(symbols)
    }

    pub async fn implementation(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<ImplementationResponse> {
        let params = ImplementationParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self
            .server
            .send_request("textDocument/implementation".to_string(), Some(params))
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let implementations = serde_json::from_value(result)?;

        Ok(implementations)
    }

    pub async fn prepare_call_hierarchy(
        &self,
        uri: String,
        line: u32,
        character: u32,
    ) -> anyhow::Result<CallHierarchyPrepareResponse> {
        let params = CallHierarchyPrepareParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self
            .server
            .send_request(
                "textDocument/prepareCallHierarchy".to_string(),
                Some(params),
            )
            .await?
            .ok_or_else(|| anyhow::anyhow!("get empty response"))?;
        let items = serde_json::from_value(result)?;

        Ok(items)
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

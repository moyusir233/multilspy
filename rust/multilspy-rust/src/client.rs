use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use super::server::RustAnalyzerServer;
use multilspy_protocol::protocol::requests::*;
use multilspy_protocol::protocol::responses::*;
use multilspy_protocol::protocol::common::*;

pub struct LspClient {
    server: RustAnalyzerServer,
}

impl LspClient {
    pub fn new(config: RustAnalyzerConfig) -> Self {
        Self {
            server: RustAnalyzerServer::new(config),
        }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.server.start().await
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        self.server.stop().await
    }

    pub async fn definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<DefinitionResponse, ServerError> {
        let params = DefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self.server.send_request("textDocument/definition".to_string(), Some(params)).await?;
        let definitions = serde_json::from_value(result)?;
        Ok(definitions)
    }

    pub async fn type_definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<TypeDefinitionResponse, ServerError> {
        let params = TypeDefinitionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self.server.send_request("textDocument/typeDefinition".to_string(), Some(params)).await?;
        let definitions = serde_json::from_value(result)?;
        Ok(definitions)
    }

    pub async fn references(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<ReferencesResponse, ServerError> {
        let params = ReferencesParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            context: ReferenceContext { include_declaration },
        };

        let result = self.server.send_request("textDocument/references".to_string(), Some(params)).await?;
        let references = serde_json::from_value(result)?;
        Ok(references)
    }

    pub async fn document_symbols(
        &mut self,
        uri: String,
    ) -> Result<DocumentSymbolResponse, ServerError> {
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
        };

        let result = self.server.send_request("textDocument/documentSymbol".to_string(), Some(params)).await?;
        let symbols = serde_json::from_value(result)?;
        Ok(symbols)
    }

    pub async fn implementation(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<ImplementationResponse, ServerError> {
        let params = ImplementationParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self.server.send_request("textDocument/implementation".to_string(), Some(params)).await?;
        let implementations = serde_json::from_value(result)?;
        Ok(implementations)
    }

    pub async fn prepare_call_hierarchy(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<CallHierarchyPrepareResponse, ServerError> {
        let params = CallHierarchyPrepareParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
        };

        let result = self.server.send_request("textDocument/prepareCallHierarchy".to_string(), Some(params)).await?;
        let items = serde_json::from_value(result)?;
        Ok(items)
    }

    pub async fn incoming_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyIncomingCallsResponse, ServerError> {
        let params = CallHierarchyIncomingCallsParams { item };

        let result = self.server.send_request("callHierarchy/incomingCalls".to_string(), Some(params)).await?;
        let calls = serde_json::from_value(result)?;
        Ok(calls)
    }

    pub async fn outgoing_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyOutgoingCallsResponse, ServerError> {
        let params = CallHierarchyOutgoingCallsParams { item };

        let result = self.server.send_request("callHierarchy/outgoingCalls".to_string(), Some(params)).await?;
        let calls = serde_json::from_value(result)?;
        Ok(calls)
    }

    pub fn is_running(&self) -> bool {
        self.server.is_running()
    }
}

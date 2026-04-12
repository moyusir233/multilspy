use super::client::LspClient;
use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use multilspy_protocol::protocol::common::*;
use multilspy_protocol::protocol::responses::*;
use std::collections::{HashSet, VecDeque};

pub struct RecursiveCallHierarchy {
    client: LspClient,
    visited: HashSet<String>,
}

impl RecursiveCallHierarchy {
    pub fn new(config: RustAnalyzerConfig) -> Self {
        Self {
            client: LspClient::new(config),
            visited: HashSet::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), ServerError> {
        self.client.start().await
    }

    pub async fn stop(&mut self) -> Result<(), ServerError> {
        self.client.stop().await
    }

    pub async fn definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<DefinitionResponse, ServerError> {
        self.client.definition(uri, line, character).await
    }

    pub async fn type_definition(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<TypeDefinitionResponse, ServerError> {
        self.client.type_definition(uri, line, character).await
    }

    pub async fn references(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        include_declaration: bool,
    ) -> Result<ReferencesResponse, ServerError> {
        self.client.references(uri, line, character, include_declaration).await
    }

    pub async fn document_symbols(
        &mut self,
        uri: String,
    ) -> Result<DocumentSymbolResponse, ServerError> {
        self.client.document_symbols(uri).await
    }

    pub async fn implementation(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<ImplementationResponse, ServerError> {
        self.client.implementation(uri, line, character).await
    }

    pub async fn prepare_call_hierarchy(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
    ) -> Result<CallHierarchyPrepareResponse, ServerError> {
        self.client.prepare_call_hierarchy(uri, line, character).await
    }

    pub async fn incoming_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyIncomingCallsResponse, ServerError> {
        self.client.incoming_calls(item).await
    }

    pub async fn outgoing_calls(
        &mut self,
        item: CallHierarchyItem,
    ) -> Result<CallHierarchyOutgoingCallsResponse, ServerError> {
        self.client.outgoing_calls(item).await
    }

    pub async fn incoming_calls_recursive(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> Result<Vec<(CallHierarchyItem, Vec<CallHierarchyIncomingCall>)>, ServerError> {
        let items = self.client.prepare_call_hierarchy(uri.clone(), line, character).await?;

        let mut results = Vec::new();
        let mut queue = VecDeque::new();

        for item in items {
            let key = format!("{}:{}:{}", item.uri, item.range.start.line, item.range.start.character);
            self.visited.insert(key);
            queue.push_back((item, 0));
        }

        while let Some((item, depth)) = queue.pop_front() {
            if let Some(max) = max_depth && depth >= max {
                continue;
            }

            let incoming_calls = self.client.incoming_calls(item.clone()).await?;

            for call in &incoming_calls {
                let key = format!("{}:{}:{}", call.from.uri, call.from.range.start.line, call.from.range.start.character);
                if !self.visited.contains(&key) {
                    self.visited.insert(key.clone());
                    queue.push_back((call.from.clone(), depth + 1));
                }
            }

            results.push((item, incoming_calls));
        }

        Ok(results)
    }

    pub async fn outgoing_calls_recursive(
        &mut self,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> Result<Vec<(CallHierarchyItem, Vec<CallHierarchyOutgoingCall>)>, ServerError> {
        let items = self.client.prepare_call_hierarchy(uri.clone(), line, character).await?;

        let mut results = Vec::new();
        let mut queue = VecDeque::new();

        for item in items {
            let key = format!("{}:{}:{}", item.uri, item.range.start.line, item.range.start.character);
            self.visited.insert(key);
            queue.push_back((item, 0));
        }

        while let Some((item, depth)) = queue.pop_front() {
            if let Some(max) = max_depth && depth >= max {
                continue;
            }

            let outgoing_calls = self.client.outgoing_calls(item.clone()).await?;

            for call in &outgoing_calls {
                let key = format!("{}:{}:{}", call.to.uri, call.to.range.start.line, call.to.range.start.character);
                if !self.visited.contains(&key) {
                    self.visited.insert(key.clone());
                    queue.push_back((call.to.clone(), depth + 1));
                }
            }

            results.push((item, outgoing_calls));
        }

        Ok(results)
    }

    pub fn clear_visited(&mut self) {
        self.visited.clear();
    }

    pub fn is_running(&self) -> bool {
        self.client.is_running()
    }
}

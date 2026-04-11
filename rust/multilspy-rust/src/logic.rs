use super::client::LspClient;
use super::config::RustAnalyzerConfig;
use super::error::ServerError;
use multilspy_protocol::protocol::common::*;
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
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
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
            if let Some(max) = max_depth {
                if depth >= max {
                    continue;
                }
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

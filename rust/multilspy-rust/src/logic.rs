use super::client::LSPClient;
use multilspy_protocol::protocol::common::*;
use std::collections::{HashSet, VecDeque};

impl LSPClient {
    async fn incoming_calls_recursive_impl(
        &self,
        visited: &mut HashSet<String>,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<(CallHierarchyItem, Vec<CallHierarchyIncomingCall>)>> {
        let items = self
            .prepare_call_hierarchy(uri.clone(), line, character)
            .await?;

        let mut results = Vec::new();
        let mut queue = VecDeque::new();

        for item in items {
            let key = format!(
                "{}:{}:{}",
                item.uri, item.range.start.line, item.range.start.character
            );
            visited.insert(key);
            queue.push_back((item, 0));
        }

        while let Some((item, depth)) = queue.pop_front() {
            if let Some(max) = max_depth
                && depth >= max
            {
                continue;
            }

            let incoming_calls = self.incoming_calls(item.clone()).await?;

            for call in &incoming_calls {
                let key = format!(
                    "{}:{}:{}",
                    call.from.uri, call.from.range.start.line, call.from.range.start.character
                );
                if !visited.contains(&key) {
                    visited.insert(key.clone());
                    queue.push_back((call.from.clone(), depth + 1));
                }
            }

            results.push((item, incoming_calls));
        }

        Ok(results)
    }

    async fn outgoing_calls_recursive_impl(
        &self,
        visited: &mut HashSet<String>,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<(CallHierarchyItem, Vec<CallHierarchyOutgoingCall>)>> {
        let items = self
            .prepare_call_hierarchy(uri.clone(), line, character)
            .await?;

        let mut results = Vec::new();
        let mut queue = VecDeque::new();

        for item in items {
            let key = format!(
                "{}:{}:{}",
                item.uri, item.range.start.line, item.range.start.character
            );
            visited.insert(key);
            queue.push_back((item, 0));
        }

        while let Some((item, depth)) = queue.pop_front() {
            if let Some(max) = max_depth
                && depth >= max
            {
                continue;
            }

            let outgoing_calls = self.outgoing_calls(item.clone()).await?;

            for call in &outgoing_calls {
                let key = format!(
                    "{}:{}:{}",
                    call.to.uri, call.to.range.start.line, call.to.range.start.character
                );
                if !visited.contains(&key) {
                    visited.insert(key.clone());
                    queue.push_back((call.to.clone(), depth + 1));
                }
            }

            results.push((item, outgoing_calls));
        }

        Ok(results)
    }
}

impl LSPClient {
    pub async fn incoming_calls_recursive(
        &self,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<(CallHierarchyItem, Vec<CallHierarchyIncomingCall>)>> {
        let mut visited = HashSet::new();
        self.incoming_calls_recursive_impl(&mut visited, uri, line, character, max_depth)
            .await
    }

    pub async fn outgoing_calls_recursive(
        &self,
        uri: String,
        line: u32,
        character: u32,
        max_depth: Option<usize>,
    ) -> anyhow::Result<Vec<(CallHierarchyItem, Vec<CallHierarchyOutgoingCall>)>> {
        let mut visited = HashSet::new();
        self.outgoing_calls_recursive_impl(&mut visited, uri, line, character, max_depth)
            .await
    }
}

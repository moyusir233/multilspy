use super::client::LSPClient;
use multilspy_protocol::protocol::common::*;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

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

    /// Analyzes dependency relationships between functions that implement the specified Rust
    /// traits within the given target directory.
    ///
    /// The `target_dir_uri` must be a `file://` URI pointing at a directory. The returned value
    /// is a JSON-ready array shape consumed by `multilspy-cli analyze-trait-impl-deps-graph`.
    pub async fn analyze_trait_impl_deps_graph(
        &self,
        trait_names: Vec<String>,
        target_dir_uri: String,
    ) -> anyhow::Result<Vec<TraitImplDepsGraphItem>> {
        analyze_trait_impl_deps_graph_impl(self, trait_names, target_dir_uri).await
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitImplDepsGraphItem {
    /// The Rust trait name that this function is considered to implement.
    pub trait_name: String,
    /// A human-readable name for the function, scoped by its `impl <Trait> for <Type>` block.
    ///
    /// This is intended for display/debugging; it is not used as an identifier in the graph.
    pub function_name: String,
    /// The `file://` URI of the file in which the function is defined.
    pub file_uri: String,
    /// The enclosing range for the function definition.
    pub range: Range,
    /// Function identifiers (see `function_id_from_range`) for target functions that are directly
    /// called by this function. Only functions within the target set are included.
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone)]
struct FunctionMeta {
    /// Stable identifier for a function node, derived from `(file_uri, range.start)`.
    id: String,
    /// The trait name that selected this function into the target set.
    trait_name: String,
    /// Human-readable name for the function node.
    function_name: String,
    /// File containing this function definition.
    file_uri: String,
    /// Enclosing range for the function definition (matches `TraitImplDepsGraphItem.range`).
    range: Range,
    /// Identifier range for the function symbol (used to position call-hierarchy queries).
    selection_range: Range,
}

/// Implementation for `LSPClient::analyze_trait_impl_deps_graph`.
///
/// The algorithm is intentionally built from existing LSP requests only:
/// - `workspace/symbol` and `workspaceSymbol/resolve` for locating trait declarations
/// - `textDocument/implementation` for locating impl blocks / impl sites
/// - `textDocument/documentSymbol` for extracting the concrete impl methods/functions
/// - `outgoing-calls-recursive` (call hierarchy) for discovering call relationships
///
/// ## Phase 1: Collect the target function set
/// 1. For each requested trait name, locate exact-match `SymbolKind::Interface` symbols.
/// 2. Resolve symbols if the server returns URI-only locations.
/// 3. For each trait symbol location, query implementations and keep only those inside
///    `target_dir_uri` (prefix match on `file://.../dir/`).
/// 4. For each implementation file, query document symbols and map impl locations to an
///    `impl <Trait> for <Type>` node, then collect all `Function`/`Method` children beneath it.
///
/// ## Phase 2: Build the dependency graph
/// - Each collected function is a node. An edge `A -> B` exists if A calls B and both are in
///   the target set.
/// - Call discovery uses `outgoing_calls_recursive` starting at each function's identifier
///   position (`selection_range.start`).
///
/// ## Why `textDocument/implementation` is used as a fallback during edge resolution
/// Rust trait dispatch can cause call hierarchy callees (`CallHierarchyOutgoingCall.to`) to
/// resolve to the *trait method declaration* rather than a concrete impl method body. In such
/// cases, a direct `(uri, range)` match against the target impl methods would miss edges.
/// The fallback uses `textDocument/implementation` at the callee position, then intersects the
/// returned locations with the target function set.
async fn analyze_trait_impl_deps_graph_impl(
    client: &LSPClient,
    trait_names: Vec<String>,
    target_dir_uri: String,
) -> anyhow::Result<Vec<TraitImplDepsGraphItem>> {
    // Normalize inputs: trim and drop empty trait names, then require at least one trait.
    let trait_names: Vec<String> = trait_names
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .collect();
    if trait_names.is_empty() {
        anyhow::bail!("at least one non-empty trait name is required");
    }

    // Directory filtering is performed with a normalized `file://.../dir/` prefix match.
    let target_dir_uri_prefix = normalize_directory_uri_prefix(target_dir_uri)?;

    // Collect implementation locations grouped by document URI.
    //
    // Each entry stores `(trait_name, impl_location)` pairs so later phases can attribute
    // extracted methods to the originating trait query.
    let mut impl_locations_by_uri: HashMap<String, Vec<(String, Location)>> = HashMap::new();

    for trait_name in &trait_names {
        // `workspace/symbol` returns a mixed list of symbols; we do a strict name + kind filter
        // to avoid accidentally selecting similarly-named items.
        let workspace_symbols = client.workspace_symbols(trait_name.clone()).await?;

        for item in workspace_symbols {
            let symbol = item.into_workspace_symbol();
            if symbol.name != *trait_name || symbol.kind != SymbolKind::Interface {
                continue;
            }

            // The server may return a URI-only location; resolve it so we can issue
            // position-based requests (implementation lookup requires a concrete position).
            let resolved = match symbol.location {
                WorkspaceSymbolLocation::Location(_) => Some(symbol),
                WorkspaceSymbolLocation::UriOnly(_) => {
                    client.workspace_symbol_resolve(symbol).await?
                }
            };

            let Some(resolved) = resolved else {
                continue;
            };
            let WorkspaceSymbolLocation::Location(location) = resolved.location else {
                continue;
            };

            // `textDocument/implementation` against the trait declaration yields impl sites.
            let implementations = client
                .implementation(
                    location.uri.clone(),
                    location.range.start.line,
                    location.range.start.character,
                )
                .await?;

            for impl_location in implementations {
                // Restrict to the target directory to avoid pulling in external crate impls.
                if !impl_location.uri.starts_with(&target_dir_uri_prefix) {
                    continue;
                }
                impl_locations_by_uri
                    .entry(impl_location.uri.clone())
                    .or_default()
                    .push((trait_name.clone(), impl_location));
            }
        }
    }

    if impl_locations_by_uri.is_empty() {
        // No trait symbols or no impls under the directory constraint.
        return Ok(Vec::new());
    }

    // Collect concrete impl methods/functions as the target node set, keyed by a stable id.
    let mut function_metas_by_id: HashMap<String, FunctionMeta> = HashMap::new();

    for (document_uri, impl_hits) in impl_locations_by_uri {
        // `textDocument/documentSymbol` gives a hierarchical symbol tree for the file.
        let document_symbols = client
            .document_symbols(document_uri.clone())
            .await
            .map_err(anyhow::Error::from)?;

        // Flatten the symbol tree to simplify matching impl blocks by location.
        let mut flattened = Vec::new();
        flatten_document_symbols(&document_symbols, &mut flattened);

        // Heuristic predicate for recognizing rust-analyzer's impl container nodes.
        //
        // rust-analyzer typically formats them like `impl <Trait> for <Type>`, but we use
        // a looser check to be resilient to small formatting differences.
        fn is_impl_node(node: &DocumentSymbolWithPath, trait_name: &str) -> bool {
            node.symbol.name.starts_with("impl") && node.symbol.name.contains(trait_name)
        }
        for (trait_name, impl_location) in impl_hits {
            let impl_nodes: Vec<&DocumentSymbolWithPath> = flattened
                .iter()
                .filter(|node| {
                    is_impl_node(node, &trait_name)
                        && range_contains_position(
                            &node.symbol.selection_range,
                            &impl_location.range.start,
                        )
                })
                .collect();

            for node in impl_nodes {
                let impl_signature = node.symbol.name.clone();
                if let Some(children) = &node.symbol.children {
                    // Collect nested function/method symbols under the impl container.
                    collect_impl_functions(
                        &trait_name,
                        &impl_signature,
                        &document_uri,
                        children,
                        &mut function_metas_by_id,
                    );
                }
            }
        }
    }

    if function_metas_by_id.is_empty() {
        // Impl blocks exist, but none yielded function/method symbols (e.g., empty impls).
        return Ok(Vec::new());
    }

    // Group target functions by URI for fast lookup when resolving call hierarchy items.
    let mut functions_by_uri: HashMap<String, Vec<FunctionMeta>> = HashMap::new();
    for meta in function_metas_by_id.values() {
        functions_by_uri
            .entry(meta.file_uri.clone())
            .or_default()
            .push(meta.clone());
    }

    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();
    for id in function_metas_by_id.keys() {
        let index = graph.add_node(id.clone());
        node_indices.insert(id.clone(), index);
    }

    // `inserted_edges` prevents duplicate edges when multiple call paths collapse to the same pair.
    let mut inserted_edges: HashSet<(NodeIndex, NodeIndex)> = HashSet::new();
    // Direct dependencies per caller id; later converted into sorted arrays for JSON output.
    let mut direct_deps: HashMap<String, HashSet<String>> = HashMap::new();
    // Cache for `textDocument/implementation` during edge resolution (keyed by uri+position).
    let mut implementation_cache: HashMap<String, Vec<Location>> = HashMap::new();

    for meta in function_metas_by_id.values() {
        let caller_index = node_indices
            .get(&meta.id)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("graph node missing for {}", meta.id))?;

        // Use the existing recursive outgoing call hierarchy implementation. The starting position
        // should be on the function identifier to maximize rust-analyzer resolution accuracy.
        let outgoing = client
            .outgoing_calls_recursive(
                meta.file_uri.clone(),
                meta.selection_range.start.line,
                meta.selection_range.start.character,
                None,
            )
            .await?;

        let mut deps_for_meta: HashSet<String> = HashSet::new();
        for (_caller, calls) in outgoing {
            for call in calls {
                // Map the callee `CallHierarchyItem` to one or more target function ids.
                //
                // The common case resolves via direct range matching. The fallback performs an
                // implementation lookup to bridge trait-method calls back to concrete impl methods.
                let callee_ids = resolve_target_function_ids_for_call(
                    client,
                    &call.to,
                    &functions_by_uri,
                    &node_indices,
                    &mut implementation_cache,
                )
                .await?;
                for callee_id in callee_ids {
                    if callee_id == meta.id {
                        continue;
                    }
                    let Some(&callee_index) = node_indices.get(&callee_id) else {
                        continue;
                    };
                    deps_for_meta.insert(callee_id.clone());
                    if inserted_edges.insert((caller_index, callee_index)) {
                        graph.add_edge(caller_index, callee_index, ());
                    }
                }
            }
        }
        direct_deps.insert(meta.id.clone(), deps_for_meta);
    }

    let mut output = Vec::with_capacity(function_metas_by_id.len());
    for meta in function_metas_by_id.values() {
        let mut deps: Vec<String> = direct_deps
            .get(&meta.id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        deps.sort();
        output.push(TraitImplDepsGraphItem {
            trait_name: meta.trait_name.clone(),
            function_name: meta.function_name.clone(),
            file_uri: meta.file_uri.clone(),
            range: meta.range.clone(),
            dependencies: deps,
        });
    }

    output.sort_by(|a, b| {
        a.file_uri
            .cmp(&b.file_uri)
            .then_with(|| a.range.start.line.cmp(&b.range.start.line))
            .then_with(|| a.range.start.character.cmp(&b.range.start.character))
            .then_with(|| a.trait_name.cmp(&b.trait_name))
            .then_with(|| a.function_name.cmp(&b.function_name))
    });

    Ok(output)
}

/// Normalizes a directory `file://` URI into a prefix suitable for `starts_with` filtering.
fn normalize_directory_uri_prefix(target_dir_uri: String) -> anyhow::Result<String> {
    let trimmed = target_dir_uri.trim();
    if trimmed.is_empty() {
        anyhow::bail!("target directory URI must not be empty");
    }
    if !trimmed.starts_with("file://") {
        anyhow::bail!("target directory must be a file:// URI, got '{}'", trimmed);
    }
    let mut prefix = trimmed.to_string();
    if !prefix.ends_with('/') {
        prefix.push('/');
    }
    Ok(prefix)
}

/// Builds a stable function identifier from a file URI and the start of the function's enclosing
/// range.
///
/// This id is used as the dependency edge payload and the `dependencies` list entries.
fn function_id_from_range(uri: &str, range: &Range) -> String {
    format!("{}#L{}:{}", uri, range.start.line, range.start.character)
}

/// Returns true if `position` lies within `range` (inclusive of both ends).
fn range_contains_position(range: &Range, position: &Position) -> bool {
    (range.start.line < position.line
        || (range.start.line == position.line && range.start.character <= position.character))
        && (position.line < range.end.line
            || (position.line == range.end.line && position.character <= range.end.character))
}

/// Fast-path mapping from a call hierarchy callee item to a target function id.
///
/// This uses the call hierarchy item's `selection_range.start` as the lookup position.
/// It returns `None` if the item URI is not in the target file set or if no target function range
/// contains the position.
fn match_function_id_for_call_hierarchy_item(
    item: &CallHierarchyItem,
    functions_by_uri: &HashMap<String, Vec<FunctionMeta>>,
) -> Option<String> {
    let metas = functions_by_uri.get(&item.uri)?;
    let position = &item.selection_range.start;
    metas
        .iter()
        .find(|meta| range_contains_position(&meta.range, position))
        .map(|meta| meta.id.clone())
}

/// Resolves the set of target function ids represented by a call hierarchy callee item.
///
/// - If the call hierarchy item already points at a concrete impl method inside the target set,
///   this returns a singleton id without additional LSP requests.
/// - Otherwise, it queries `textDocument/implementation` at the callee position and intersects
///   those impl locations with the target set. This is required for trait dispatch patterns where
///   the call hierarchy callee points at a trait method declaration rather than an impl body.
async fn resolve_target_function_ids_for_call(
    client: &LSPClient,
    item: &CallHierarchyItem,
    functions_by_uri: &HashMap<String, Vec<FunctionMeta>>,
    node_indices: &HashMap<String, NodeIndex>,
    implementation_cache: &mut HashMap<String, Vec<Location>>,
) -> anyhow::Result<Vec<String>> {
    if let Some(id) = match_function_id_for_call_hierarchy_item(item, functions_by_uri)
        && node_indices.contains_key(&id)
    {
        return Ok(vec![id]);
    }

    let key = format!(
        "{}:{}:{}",
        item.uri, item.selection_range.start.line, item.selection_range.start.character
    );
    let locations = if let Some(cached) = implementation_cache.get(&key) {
        cached.clone()
    } else {
        let fetched = client
            .implementation(
                item.uri.clone(),
                item.selection_range.start.line,
                item.selection_range.start.character,
            )
            .await?;
        implementation_cache.insert(key, fetched.clone());
        fetched
    };

    let mut ids = HashSet::new();
    for location in locations {
        let Some(metas) = functions_by_uri.get(&location.uri) else {
            continue;
        };
        for meta in metas {
            if range_contains_position(&meta.range, &location.range.start) {
                ids.insert(meta.id.clone());
            }
        }
    }

    let mut resolved: Vec<String> = ids
        .into_iter()
        .filter(|id| node_indices.contains_key(id))
        .collect();
    resolved.sort();
    Ok(resolved)
}

#[derive(Debug, Clone)]
struct DocumentSymbolWithPath {
    symbol: DocumentSymbol,
}

/// Flattens a hierarchical `DocumentSymbol` tree into a simple list while preserving each node's
/// symbol data.
fn flatten_document_symbols(symbols: &[DocumentSymbol], out: &mut Vec<DocumentSymbolWithPath>) {
    for symbol in symbols {
        out.push(DocumentSymbolWithPath {
            symbol: symbol.clone(),
        });

        if let Some(children) = &symbol.children {
            flatten_document_symbols(children, out);
        }
    }
}

/// Collects function/method symbols under an `impl <Trait> for <Type>` container node into the
/// target function set.
///
/// Each collected function becomes a graph node candidate. The `trait_name` is recorded so output
/// items can report which trait query selected the function.
fn collect_impl_functions(
    trait_name: &str,
    impl_signature: &str,
    document_uri: &str,
    children: &[DocumentSymbol],
    out: &mut HashMap<String, FunctionMeta>,
) {
    for child in children {
        if matches!(child.kind, SymbolKind::Function | SymbolKind::Method) {
            let id = function_id_from_range(document_uri, &child.range);
            out.entry(id.clone()).or_insert_with(|| FunctionMeta {
                id,
                trait_name: trait_name.to_string(),
                function_name: format!("{}::{}", impl_signature, child.name),
                file_uri: document_uri.to_string(),
                range: child.range.clone(),
                selection_range: child.selection_range.clone(),
            });
        }

        if let Some(grandchildren) = &child.children {
            collect_impl_functions(trait_name, impl_signature, document_uri, grandchildren, out);
        }
    }
}

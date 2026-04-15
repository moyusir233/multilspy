use super::client::LSPClient;
use multilspy_protocol::protocol::common::*;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
struct DocumentSymbolMatchEntry {
    name: String,
    range: Range,
    selection_range: Range,
}

fn flatten_document_symbols_with_parent_kinds(
    symbols: &[DocumentSymbol],
    entries: &mut Vec<DocumentSymbolMatchEntry>,
) {
    for symbol in symbols {
        entries.push(DocumentSymbolMatchEntry {
            name: symbol.name.clone(),
            range: symbol.range.clone(),
            selection_range: symbol.selection_range.clone(),
        });

        if let Some(children) = &symbol.children {
            flatten_document_symbols_with_parent_kinds(children, entries);
        }
    }
}

fn select_document_symbol_entry<'a>(
    entries: &'a [DocumentSymbolMatchEntry],
    name: &str,
    position: &Position,
) -> Option<&'a DocumentSymbolMatchEntry> {
    let selection_range_matches: Vec<_> = entries
        .iter()
        .filter(|entry| {
            entry.name == name && range_contains_position(&entry.selection_range, position)
        })
        .collect();
    if selection_range_matches.len() == 1 {
        return selection_range_matches.into_iter().next();
    }

    let range_matches: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name == name && range_contains_position(&entry.range, position))
        .collect();
    if range_matches.len() == 1 {
        return range_matches.into_iter().next();
    }

    let name_matches: Vec<_> = entries.iter().filter(|entry| entry.name == name).collect();
    if name_matches.len() == 1 {
        return name_matches.into_iter().next();
    }

    None
}

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

    /// Analyzes trait-method dependencies reached from exactly one resolved entry function.
    ///
    /// The returned value is a JSON-ready array shape consumed by
    /// `multilspy-cli analyze-fn-call-trait-deps-graph`.
    pub async fn analyze_fn_call_trait_deps_graph(
        &self,
        entry_uri: String,
        line: u32,
        character: u32,
        trait_names: Vec<String>,
        target_dir_uris: Vec<String>,
    ) -> anyhow::Result<Vec<FnCallTraitDepsGraphItem>> {
        analyze_fn_call_trait_deps_graph_impl(
            self,
            entry_uri,
            line,
            character,
            trait_names,
            target_dir_uris,
        )
        .await
    }

    /// Analyzes dependency relationships between functions that implement the specified Rust
    /// traits within the given target directory.
    ///
    /// The `target_dir_uris` must contain one or more `file://` URIs pointing at directories.
    /// The returned value is a JSON-ready array shape consumed by
    /// `multilspy-cli analyze-trait-impl-deps-graph`.
    pub async fn analyze_trait_impl_deps_graph(
        &self,
        trait_names: Vec<String>,
        target_dir_uris: Vec<String>,
    ) -> anyhow::Result<Vec<TraitImplDepsGraphItem>> {
        analyze_trait_impl_deps_graph_impl(self, trait_names, target_dir_uris).await
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
    /// Target functions that are directly called by this function.
    ///
    /// Each dependency is described using the callee's `trait_name`, `file_uri`, and
    /// `function_name`, which is more stable for downstream consumers than a range-based id.
    pub dependencies: Vec<TraitImplDepsGraphDependency>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TraitImplDepsGraphDependency {
    pub trait_name: String,
    pub file_uri: String,
    pub function_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FnCallTraitDepsGraphItem {
    pub function_name: String,
    pub entry_uri: String,
    pub entry_position: Position,
    pub file_uri: String,
    pub range: Range,
    pub dependencies: Vec<FnCallTraitDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FnCallTraitDependency {
    pub dependency_id: String,
    #[serde(rename = "callStack")]
    pub call_stack: Vec<String>,
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

#[derive(Debug, Clone)]
struct ResolvedWorkspaceSymbol {
    symbol: WorkspaceSymbol,
    location: Location,
}

#[derive(Debug, Clone)]
struct ImplMethodMeta {
    id: String,
    range: Range,
}

#[derive(Debug, Clone)]
struct EntryFunctionMeta {
    function_name: String,
    file_uri: String,
    range: Range,
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
/// 3. For each trait symbol location, query implementations and keep only those inside any
///    requested target directory (prefix match on `file://.../dir/`).
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
    target_dir_uris: Vec<String>,
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

    // Directory filtering is performed with normalized `file://.../dir/` prefix matches.
    let target_dir_uri_prefixes = normalize_directory_uri_prefixes(target_dir_uris)?;

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
                // Restrict to the requested target directories to avoid pulling in external
                // crate impls or unrelated workspace files.
                if !target_dir_uri_prefixes
                    .iter()
                    .any(|prefix| impl_location.uri.starts_with(prefix))
                {
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
                if let Some(children) = &node.symbol.children {
                    // Collect nested function/method symbols under the impl container.
                    collect_impl_functions(
                        &trait_name,
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
        let mut deps: Vec<TraitImplDepsGraphDependency> = direct_deps
            .get(&meta.id)
            .map(|set| {
                set.iter()
                    .filter_map(|id| function_metas_by_id.get(id))
                    .map(|callee| TraitImplDepsGraphDependency {
                        trait_name: callee.trait_name.clone(),
                        file_uri: callee.file_uri.clone(),
                        function_name: callee.function_name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        deps.sort_by(|a, b| {
            a.trait_name
                .cmp(&b.trait_name)
                .then_with(|| a.file_uri.cmp(&b.file_uri))
                .then_with(|| a.function_name.cmp(&b.function_name))
        });
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

async fn analyze_fn_call_trait_deps_graph_impl(
    client: &LSPClient,
    entry_uri: String,
    line: u32,
    character: u32,
    trait_names: Vec<String>,
    target_dir_uris: Vec<String>,
) -> anyhow::Result<Vec<FnCallTraitDepsGraphItem>> {
    let entry_uri = entry_uri.trim().to_string();
    if entry_uri.is_empty() {
        anyhow::bail!("entry function URI must not be empty");
    }
    if !entry_uri.starts_with("file://") {
        anyhow::bail!("entry function URI must be a file:// URI, got '{}'", entry_uri);
    }
    let entry_position = Position { line, character };

    let mut seen_trait_names = HashSet::new();
    let trait_names: Vec<String> = trait_names
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
        .filter(|name| seen_trait_names.insert(name.clone()))
        .collect();
    if trait_names.is_empty() {
        anyhow::bail!("at least one non-empty trait name is required");
    }

    let target_dir_uri_prefixes = normalize_directory_uri_prefixes(target_dir_uris)?;
    if !target_dir_uri_prefixes
        .iter()
        .any(|prefix| entry_uri.starts_with(prefix))
    {
        anyhow::bail!("entry function URI '{}' is not inside the target directory", entry_uri);
    }

    let entry_function =
        resolve_entry_function_at_position(client, &entry_uri, &entry_position).await?;

    let mut document_symbol_cache = HashMap::new();
    let mut document_symbol_entry_cache = HashMap::new();

    let mut target_impl_methods_by_uri: HashMap<String, Vec<ImplMethodMeta>> = HashMap::new();
    let mut dependency_ids_by_impl_method_id: HashMap<String, HashSet<String>> = HashMap::new();

    for trait_name in trait_names {
        let resolved_trait = resolve_exact_workspace_symbol(
            client,
            &trait_name,
            &[SymbolKind::Interface],
            Some(&target_dir_uri_prefixes),
            "trait",
        )
        .await?;

        let trait_document_symbols = get_document_symbols_cached(
            client,
            &resolved_trait.location.uri,
            &mut document_symbol_cache,
        )
        .await?;
        let Some(trait_symbol) = select_document_symbol_node(
            &trait_document_symbols,
            &resolved_trait.symbol.name,
            &resolved_trait.location.range.start,
        ) else {
            continue;
        };

        let mut trait_methods = Vec::new();
        if let Some(children) = &trait_symbol.children {
            collect_function_like_symbols(children, &mut trait_methods);
        }

        for trait_method in trait_methods {
            let dependency_id = format!("{}.{}", resolved_trait.symbol.name, trait_method.name);
            let implementations = client
                .implementation(
                    resolved_trait.location.uri.clone(),
                    trait_method.selection_range.start.line,
                    trait_method.selection_range.start.character,
                )
                .await?;

            for implementation in implementations {
                if !target_dir_uri_prefixes
                    .iter()
                    .any(|prefix| implementation.uri.starts_with(prefix))
                {
                    continue;
                }

                let Some(impl_method_symbol) = resolve_document_symbol_entry(
                    client,
                    &implementation,
                    &trait_method.name,
                    &mut document_symbol_entry_cache,
                )
                .await?
                else {
                    continue;
                };

                let impl_method_id =
                    function_id_from_range(&implementation.uri, &impl_method_symbol.range);
                dependency_ids_by_impl_method_id
                    .entry(impl_method_id.clone())
                    .or_default()
                    .insert(dependency_id.clone());
                target_impl_methods_by_uri
                    .entry(implementation.uri.clone())
                    .or_default()
                    .push(ImplMethodMeta {
                        id: impl_method_id,
                        range: impl_method_symbol.range.clone(),
                    });
            }
        }
    }

    for impl_methods in target_impl_methods_by_uri.values_mut() {
        impl_methods.sort_by(|a, b| a.id.cmp(&b.id));
        impl_methods.dedup_by(|a, b| a.id == b.id);
    }

    let root_items = client
        .prepare_call_hierarchy(
            entry_function.file_uri.clone(),
            entry_function.selection_range.start.line,
            entry_function.selection_range.start.character,
        )
        .await?;
    if root_items.is_empty() {
        anyhow::bail!(
            "failed to resolve call hierarchy root for entry function '{}'",
            entry_function.function_name
        );
    }

    let mut call_graph: DiGraph<String, ()> = DiGraph::new();
    let mut call_graph_node_indices: HashMap<String, NodeIndex> = HashMap::new();
    let mut call_graph_display_names: HashMap<String, String> = HashMap::new();
    let mut inserted_edges: HashSet<(NodeIndex, NodeIndex)> = HashSet::new();
    let mut root_call_node_ids = Vec::new();
    for root_item in &root_items {
        let root_id = call_hierarchy_item_id(root_item);
        let root_index = ensure_call_graph_node(
            &mut call_graph,
            &mut call_graph_node_indices,
            &mut call_graph_display_names,
            &root_id,
            &root_item.name,
        );
        let _ = root_index;
        root_call_node_ids.push(root_id);
    }

    let mut dependency_target_call_nodes: HashMap<String, HashSet<String>> = HashMap::new();
    let mut implementation_cache = HashMap::new();

    let outgoing = client
        .outgoing_calls_recursive(
            entry_function.file_uri.clone(),
            entry_function.selection_range.start.line,
            entry_function.selection_range.start.character,
            None,
        )
        .await?;

    for (_caller, calls) in outgoing {
        let caller_id = call_hierarchy_item_id(&_caller);
        let caller_index = ensure_call_graph_node(
            &mut call_graph,
            &mut call_graph_node_indices,
            &mut call_graph_display_names,
            &caller_id,
            &_caller.name,
        );
        for call in calls {
            let callee_id = call_hierarchy_item_id(&call.to);
            let callee_index = ensure_call_graph_node(
                &mut call_graph,
                &mut call_graph_node_indices,
                &mut call_graph_display_names,
                &callee_id,
                &call.to.name,
            );
            if inserted_edges.insert((caller_index, callee_index)) {
                call_graph.add_edge(caller_index, callee_index, ());
            }

            let dependency_ids = resolve_dependency_ids_for_call(
                client,
                &call.to,
                &target_impl_methods_by_uri,
                &dependency_ids_by_impl_method_id,
                &mut implementation_cache,
            )
            .await?;

            for dependency_id in dependency_ids {
                dependency_target_call_nodes
                    .entry(dependency_id)
                    .or_default()
                    .insert(callee_id.clone());
            }
        }
    }

    let mut dependency_ids: Vec<String> = dependency_target_call_nodes.keys().cloned().collect();
    dependency_ids.sort();
    let dependencies = dependency_ids
        .into_iter()
        .map(|dependency_id| {
            let target_nodes = dependency_target_call_nodes
                .get(&dependency_id)
                .cloned()
                .unwrap_or_default();
            FnCallTraitDependency {
                call_stack: find_representative_call_stack(
                    &call_graph,
                    &call_graph_node_indices,
                    &call_graph_display_names,
                    &root_call_node_ids,
                    &target_nodes,
                    &entry_function.function_name,
                    &dependency_id,
                ),
                dependency_id,
            }
        })
        .collect();

    Ok(vec![FnCallTraitDepsGraphItem {
        function_name: entry_function.function_name.clone(),
        entry_uri,
        entry_position,
        file_uri: entry_function.file_uri.clone(),
        range: entry_function.range.clone(),
        dependencies,
    }])
}

/// Normalizes directory `file://` URIs into prefixes suitable for `starts_with` filtering.
fn normalize_directory_uri_prefixes(target_dir_uris: Vec<String>) -> anyhow::Result<Vec<String>> {
    if target_dir_uris.is_empty() {
        anyhow::bail!("at least one target directory URI must be provided");
    }

    target_dir_uris
        .into_iter()
        .map(|target_dir_uri| {
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
        })
        .collect()
}

async fn resolve_exact_workspace_symbol(
    client: &LSPClient,
    query: &str,
    allowed_kinds: &[SymbolKind],
    target_dir_uri_prefixes: Option<&[String]>,
    entity_name: &str,
) -> anyhow::Result<ResolvedWorkspaceSymbol> {
    let mut matches = Vec::new();
    for item in client.workspace_symbols(query.to_string()).await? {
        let symbol = item.into_workspace_symbol();
        if symbol.name != query {
            continue;
        }
        if !allowed_kinds.is_empty() && !allowed_kinds.contains(&symbol.kind) {
            continue;
        }

        let Some(location) = resolve_workspace_symbol_location(client, symbol.clone()).await? else {
            continue;
        };
        if let Some(prefixes) = target_dir_uri_prefixes
            && !prefixes.iter().any(|prefix| location.uri.starts_with(prefix))
        {
            continue;
        }

        matches.push(ResolvedWorkspaceSymbol { symbol, location });
    }

    if matches.is_empty() {
        let scope_suffix = if target_dir_uri_prefixes.is_some() {
            " in the target directory"
        } else {
            ""
        };
        anyhow::bail!("{} '{}' not found{}", entity_name, query, scope_suffix);
    }
    if matches.len() > 1 {
        anyhow::bail!("{} '{}' resolved to multiple symbols", entity_name, query);
    }

    Ok(matches.remove(0))
}

async fn resolve_workspace_symbol_location(
    client: &LSPClient,
    symbol: WorkspaceSymbol,
) -> anyhow::Result<Option<Location>> {
    let resolved_symbol = match symbol.location {
        WorkspaceSymbolLocation::Location(location) => {
            return Ok(Some(location));
        }
        WorkspaceSymbolLocation::UriOnly(_) => client.workspace_symbol_resolve(symbol).await?,
    };

    let Some(resolved_symbol) = resolved_symbol else {
        return Ok(None);
    };
    match resolved_symbol.location {
        WorkspaceSymbolLocation::Location(location) => Ok(Some(location)),
        WorkspaceSymbolLocation::UriOnly(_) => Ok(None),
    }
}

async fn resolve_entry_function_at_position(
    client: &LSPClient,
    entry_uri: &str,
    entry_position: &Position,
) -> anyhow::Result<EntryFunctionMeta> {
    let document_symbols = client.document_symbols(entry_uri.to_string()).await?;
    let mut flattened = Vec::new();
    flatten_document_symbols(&document_symbols, &mut flattened);

    let mut matches: Vec<&DocumentSymbolWithPath> = flattened
        .iter()
        .filter(|symbol| {
            matches!(symbol.symbol.kind, SymbolKind::Function | SymbolKind::Method)
                && range_contains_position(&symbol.symbol.range, entry_position)
        })
        .collect();

    if matches.is_empty() {
        anyhow::bail!(
            "entry position {}:{} in '{}' is not inside any function or method",
            entry_position.line,
            entry_position.character,
            entry_uri
        );
    }

    matches.sort_by(|a, b| {
        let a_span = (
            a.symbol.range.end.line.saturating_sub(a.symbol.range.start.line),
            a.symbol
                .range
                .end
                .character
                .saturating_sub(a.symbol.range.start.character),
        );
        let b_span = (
            b.symbol.range.end.line.saturating_sub(b.symbol.range.start.line),
            b.symbol
                .range
                .end
                .character
                .saturating_sub(b.symbol.range.start.character),
        );
        a_span
            .cmp(&b_span)
            .then_with(|| a.symbol.range.start.line.cmp(&b.symbol.range.start.line))
            .then_with(|| a.symbol.range.start.character.cmp(&b.symbol.range.start.character))
    });

    let best = matches[0];
    if matches.len() > 1 {
        let second = matches[1];
        if best.symbol.range == second.symbol.range {
            anyhow::bail!(
                "entry position {}:{} in '{}' resolves ambiguously to multiple function-like symbols",
                entry_position.line,
                entry_position.character,
                entry_uri
            );
        }
    }

    Ok(EntryFunctionMeta {
        function_name: best.symbol.name.clone(),
        file_uri: entry_uri.to_string(),
        range: best.symbol.range.clone(),
        selection_range: best.symbol.selection_range.clone(),
    })
}

/// Builds a stable function identifier from a file URI and the start of the function's enclosing
/// range.
///
/// This id is used internally as the dependency edge payload and for set membership checks.
fn function_id_from_range(uri: &str, range: &Range) -> String {
    format!("{}#L{}:{}", uri, range.start.line, range.start.character)
}

fn call_hierarchy_item_id(item: &CallHierarchyItem) -> String {
    format!(
        "{}#L{}:{}",
        item.uri, item.range.start.line, item.range.start.character
    )
}

/// Returns true if `position` lies within `range` (inclusive of both ends).
fn range_contains_position(range: &Range, position: &Position) -> bool {
    (range.start.line < position.line
        || (range.start.line == position.line && range.start.character <= position.character))
        && (position.line < range.end.line
            || (position.line == range.end.line && position.character <= range.end.character))
}

fn ensure_call_graph_node(
    graph: &mut DiGraph<String, ()>,
    node_indices: &mut HashMap<String, NodeIndex>,
    display_names: &mut HashMap<String, String>,
    node_id: &str,
    display_name: &str,
) -> NodeIndex {
    if let Some(index) = node_indices.get(node_id).copied() {
        display_names
            .entry(node_id.to_string())
            .or_insert_with(|| display_name.to_string());
        return index;
    }

    let index = graph.add_node(node_id.to_string());
    node_indices.insert(node_id.to_string(), index);
    display_names.insert(node_id.to_string(), display_name.to_string());
    index
}

fn find_representative_call_stack(
    graph: &DiGraph<String, ()>,
    node_indices: &HashMap<String, NodeIndex>,
    display_names: &HashMap<String, String>,
    root_node_ids: &[String],
    target_node_ids: &HashSet<String>,
    entry_function_name: &str,
    dependency_id: &str,
) -> Vec<String> {
    let target_indices: HashSet<NodeIndex> = target_node_ids
        .iter()
        .filter_map(|node_id| node_indices.get(node_id).copied())
        .collect();
    if target_indices.is_empty() {
        return vec![entry_function_name.to_string(), dependency_id.to_string()];
    }

    let mut queue = VecDeque::new();
    let mut predecessors: HashMap<NodeIndex, Option<NodeIndex>> = HashMap::new();
    for root_id in root_node_ids {
        let Some(root_index) = node_indices.get(root_id).copied() else {
            continue;
        };
        if predecessors.insert(root_index, None).is_none() {
            queue.push_back(root_index);
        }
    }

    let mut found_target = None;
    while let Some(node_index) = queue.pop_front() {
        if target_indices.contains(&node_index) {
            found_target = Some(node_index);
            break;
        }

        for neighbor in graph.neighbors(node_index) {
            if predecessors.contains_key(&neighbor) {
                continue;
            }
            predecessors.insert(neighbor, Some(node_index));
            queue.push_back(neighbor);
        }
    }

    let Some(mut current) = found_target else {
        return vec![entry_function_name.to_string(), dependency_id.to_string()];
    };

    let mut reversed_indices = Vec::new();
    loop {
        reversed_indices.push(current);
        match predecessors.get(&current).copied().flatten() {
            Some(prev) => current = prev,
            None => break,
        }
    }
    reversed_indices.reverse();

    let mut call_stack: Vec<String> = reversed_indices
        .into_iter()
        .filter_map(|node_index| graph.node_weight(node_index))
        .map(|node_id| {
            display_names
                .get(node_id)
                .cloned()
                .unwrap_or_else(|| node_id.clone())
        })
        .collect();

    if call_stack.is_empty() {
        call_stack.push(entry_function_name.to_string());
    } else if call_stack[0] != entry_function_name {
        call_stack[0] = entry_function_name.to_string();
    }

    match call_stack.last_mut() {
        Some(last) => *last = dependency_id.to_string(),
        None => call_stack.push(dependency_id.to_string()),
    }
    call_stack
}

async fn get_document_symbols_cached(
    client: &LSPClient,
    uri: &str,
    cache: &mut HashMap<String, Vec<DocumentSymbol>>,
) -> anyhow::Result<Vec<DocumentSymbol>> {
    if let Some(symbols) = cache.get(uri) {
        return Ok(symbols.clone());
    }

    let symbols = client.document_symbols(uri.to_string()).await?;
    cache.insert(uri.to_string(), symbols.clone());
    Ok(symbols)
}

async fn resolve_document_symbol_entry(
    client: &LSPClient,
    location: &Location,
    symbol_name: &str,
    cache: &mut HashMap<String, Vec<DocumentSymbolMatchEntry>>,
) -> anyhow::Result<Option<DocumentSymbolMatchEntry>> {
    if !cache.contains_key(&location.uri) {
        let document_symbols = client.document_symbols(location.uri.clone()).await?;
        let mut flattened_entries = Vec::new();
        flatten_document_symbols_with_parent_kinds(&document_symbols, &mut flattened_entries);
        cache.insert(location.uri.clone(), flattened_entries);
    }

    Ok(cache.get(&location.uri).and_then(|entries| {
        select_document_symbol_entry(entries, symbol_name, &location.range.start).cloned()
    }))
}

fn select_document_symbol_node(
    document_symbols: &[DocumentSymbol],
    name: &str,
    position: &Position,
) -> Option<DocumentSymbol> {
    let mut flattened = Vec::new();
    flatten_document_symbols(document_symbols, &mut flattened);

    let selection_range_matches: Vec<_> = flattened
        .iter()
        .filter(|symbol| {
            symbol.symbol.name == name
                && range_contains_position(&symbol.symbol.selection_range, position)
        })
        .collect();
    if selection_range_matches.len() == 1 {
        return selection_range_matches
            .into_iter()
            .next()
            .map(|symbol| symbol.symbol.clone());
    }

    let range_matches: Vec<_> = flattened
        .iter()
        .filter(|symbol| {
            symbol.symbol.name == name && range_contains_position(&symbol.symbol.range, position)
        })
        .collect();
    if range_matches.len() == 1 {
        return range_matches
            .into_iter()
            .next()
            .map(|symbol| symbol.symbol.clone());
    }

    let name_matches: Vec<_> = flattened
        .iter()
        .filter(|symbol| symbol.symbol.name == name)
        .collect();
    if name_matches.len() == 1 {
        return name_matches
            .into_iter()
            .next()
            .map(|symbol| symbol.symbol.clone());
    }

    None
}

fn collect_function_like_symbols(symbols: &[DocumentSymbol], out: &mut Vec<DocumentSymbol>) {
    for symbol in symbols {
        if matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
            out.push(symbol.clone());
        }
        if let Some(children) = &symbol.children {
            collect_function_like_symbols(children, out);
        }
    }
}

fn match_impl_method_ids_at_position(
    uri: &str,
    position: &Position,
    target_impl_methods_by_uri: &HashMap<String, Vec<ImplMethodMeta>>,
) -> Vec<String> {
    let Some(impl_methods) = target_impl_methods_by_uri.get(uri) else {
        return Vec::new();
    };

    let mut matched_ids: Vec<String> = impl_methods
        .iter()
        .filter(|impl_method| range_contains_position(&impl_method.range, position))
        .map(|impl_method| impl_method.id.clone())
        .collect();
    matched_ids.sort();
    matched_ids.dedup();
    matched_ids
}

async fn resolve_dependency_ids_for_call(
    client: &LSPClient,
    item: &CallHierarchyItem,
    target_impl_methods_by_uri: &HashMap<String, Vec<ImplMethodMeta>>,
    dependency_ids_by_impl_method_id: &HashMap<String, HashSet<String>>,
    implementation_cache: &mut HashMap<String, Vec<Location>>,
) -> anyhow::Result<Vec<String>> {
    let direct_impl_method_ids = match_impl_method_ids_at_position(
        &item.uri,
        &item.selection_range.start,
        target_impl_methods_by_uri,
    );
    let mut dependency_ids = HashSet::new();
    for impl_method_id in direct_impl_method_ids {
        if let Some(ids) = dependency_ids_by_impl_method_id.get(&impl_method_id) {
            dependency_ids.extend(ids.iter().cloned());
        }
    }
    if !dependency_ids.is_empty() {
        let mut resolved: Vec<String> = dependency_ids.into_iter().collect();
        resolved.sort();
        return Ok(resolved);
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

    for location in locations {
        for impl_method_id in match_impl_method_ids_at_position(
            &location.uri,
            &location.range.start,
            target_impl_methods_by_uri,
        ) {
            if let Some(ids) = dependency_ids_by_impl_method_id.get(&impl_method_id) {
                dependency_ids.extend(ids.iter().cloned());
            }
        }
    }

    let mut resolved: Vec<String> = dependency_ids.into_iter().collect();
    resolved.sort();
    Ok(resolved)
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
                function_name: child.name.clone(),
                file_uri: document_uri.to_string(),
                range: child.range.clone(),
                selection_range: child.selection_range.clone(),
            });
        }

        if let Some(grandchildren) = &child.children {
            collect_impl_functions(trait_name, document_uri, grandchildren, out);
        }
    }
}

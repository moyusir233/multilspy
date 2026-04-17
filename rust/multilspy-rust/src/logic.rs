use super::client::LSPClient;
use fluent_uri::Uri;
use multilspy_protocol::protocol::common::*;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

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

    /// Analyzes dependency relationships between a mixed set of targets, each of which can carry
    /// arbitrary user-provided metadata that is preserved in the final output items.
    pub async fn analyze_func_deps_graph_with_targets(
        &self,
        params: AnalyzeFuncDepsGraphParams,
    ) -> anyhow::Result<Vec<AnalyzeFuncDepsGraphItem>> {
        analyze_func_deps_graph_impl(self, params).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyzeFuncDepsGraphParams {
    pub targets: Vec<AnalyzeFuncDepsGraphTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "target_type", rename_all = "snake_case")]
pub enum AnalyzeFuncDepsGraphTarget {
    TraitImpl {
        trait_name: String,
        target_dir_uri: String,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        extra: HashMap<String, Value>,
    },
    RegularFunction {
        file_uri: String,
        line: u32,
        character: u32,
        #[serde(default, skip_serializing_if = "HashMap::is_empty")]
        extra: HashMap<String, Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AnalyzeFuncDepsGraphFnType {
    TraitImpl,
    RegularFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeFuncDepsGraphItem {
    pub fn_type: AnalyzeFuncDepsGraphFnType,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
    /// A human-readable name for the function.
    pub function_name: String,
    /// The `file://` URI of the file in which the function is defined.
    pub file_uri: String,
    /// The enclosing range for the function definition.
    pub range: Range,
    /// Target functions that this function depends on anywhere in its reachable outgoing call
    /// chain. This keeps the previous recursive dependency semantics intact.
    pub dependencies: Vec<AnalyzeFuncDepsGraphDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyzeFuncDepsGraphDependency {
    pub fn_type: AnalyzeFuncDepsGraphFnType,
    pub file_uri: String,
    pub function_name: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
struct FunctionMeta {
    /// Stable identifier for a function node, derived from `(file_uri, range.start)`.
    id: String,
    /// The category of function represented by this node.
    fn_type: AnalyzeFuncDepsGraphFnType,
    /// Final output metadata for this function. For trait impl targets this includes the trait
    /// name, and for both target kinds it also preserves any arbitrary user-provided metadata.
    extra: HashMap<String, Value>,
    /// Human-readable name for the function node.
    function_name: String,
    /// File containing this function definition.
    file_uri: String,
    /// Enclosing range for the function definition (matches `AnalyzeFuncDepsGraphItem.range`).
    range: Range,
    /// Identifier range for the function symbol (used to position call-hierarchy queries).
    selection_range: Range,
}

type ExtraMetadata = HashMap<String, Value>;
type TraitTargetDirEntry = (String, ExtraMetadata);
type TraitImplLocationEntry = (String, Location, ExtraMetadata);

struct ParseDepsContext<'a> {
    graph: &'a mut DiGraph<String, ()>,
    node_indices: &'a HashMap<String, NodeIndex>,
    functions_by_uri: &'a HashMap<String, Vec<FunctionMeta>>,
    target_function_names: &'a HashSet<String>,
    implementation_cache: &'a mut HashMap<String, Vec<Location>>,
}

/// Implementation for `LSPClient::analyze_func_deps_graph`.
///
/// The algorithm is intentionally built from existing LSP requests only:
/// - `workspace/symbol` and `workspaceSymbol/resolve` for locating trait declarations
/// - `textDocument/implementation` for locating impl blocks / impl sites
/// - `textDocument/documentSymbol` for extracting the concrete impl methods/functions
/// - `outgoing_calls_recursive` for discovering the full reachable call chain
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
/// - Call discovery uses the existing `outgoing_calls_recursive` workflow so the analysis keeps
///   the previous transitive-dependency semantics: if a selected function reaches another selected
///   function anywhere in its outgoing call chain, the dependency is recorded.
///
/// ## Why `textDocument/implementation` is used as a fallback during edge resolution
/// Rust trait dispatch can cause call hierarchy callees (`CallHierarchyOutgoingCall.to`) to
/// resolve to the *trait method declaration* rather than a concrete impl method body. In such
/// cases, a direct `(uri, range)` match against the target impl methods would miss edges.
/// The fallback uses `textDocument/implementation` at the callee position, then intersects the
/// returned locations with the target function set.
///
/// ## Why the fallback is selective
/// Recursive call hierarchy can quickly walk into library or dependency code. We only invoke the
/// expensive `textDocument/implementation` fallback when the callee name could plausibly map back
/// to one of the selected target functions. This preserves the original trait-dispatch fix while
/// avoiding unnecessary lookups for obviously unrelated external functions.
async fn analyze_func_deps_graph_impl(
    client: &LSPClient,
    params: AnalyzeFuncDepsGraphParams,
) -> anyhow::Result<Vec<AnalyzeFuncDepsGraphItem>> {
    if params.targets.is_empty() {
        anyhow::bail!("at least one analysis target is required");
    }

    // Directory filtering is performed with normalized `file://.../dir/` prefix matches.
    let mut target_dir_uri_prefixes_by_trait: HashMap<String, Vec<TraitTargetDirEntry>> =
        HashMap::new();
    let mut regular_function_targets = Vec::new();
    for target in params.targets {
        match target {
            AnalyzeFuncDepsGraphTarget::TraitImpl {
                trait_name,
                target_dir_uri,
                extra,
            } => {
                let trait_name = trait_name.trim().to_string();
                if trait_name.is_empty() {
                    anyhow::bail!("trait implementation target requires a non-empty trait name");
                }
                let prefix = normalize_directory_uri_prefix(target_dir_uri)?;
                target_dir_uri_prefixes_by_trait
                    .entry(trait_name)
                    .or_default()
                    .push((prefix, extra));
            }
            AnalyzeFuncDepsGraphTarget::RegularFunction {
                file_uri,
                line,
                character,
                extra,
            } => {
                parse_file_uri(&file_uri)?;
                regular_function_targets.push((file_uri, line, character, extra));
            }
        }
    }

    // Collect implementation locations grouped by document URI.
    //
    // Each entry stores `(trait_name, impl_location)` pairs so later phases can attribute
    // extracted methods to the originating trait query.
    let mut trait_impl_locations_by_uri: HashMap<String, Vec<TraitImplLocationEntry>> =
        HashMap::new();

    for (trait_name, target_dir_uri_prefixes) in &target_dir_uri_prefixes_by_trait {
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
                let matching_target = target_dir_uri_prefixes
                    .iter()
                    .find(|(prefix, _)| impl_location.uri.starts_with(prefix));
                let Some((_, matching_extra)) = matching_target else {
                    continue;
                };
                trait_impl_locations_by_uri
                    .entry(impl_location.uri.clone())
                    .or_default()
                    .push((trait_name.clone(), impl_location, matching_extra.clone()));
            }
        }
    }

    // Collect concrete impl methods/functions of trait as the target node set, keyed by a stable id.
    let mut function_metas_by_id: HashMap<String, FunctionMeta> = HashMap::new();

    for (document_uri, impl_hits) in trait_impl_locations_by_uri {
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
            node.symbol.name.trim().starts_with("impl") && node.symbol.name.contains(trait_name)
        }
        for (trait_name, impl_location, extra) in impl_hits {
            // find impl nodes which match trait impl location
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
                    // Collect nested function/method symbols under the trait impl container.
                    collect_impl_functions(
                        &trait_name,
                        &document_uri,
                        children,
                        &extra,
                        &mut function_metas_by_id,
                    );
                }
            }
        }
    }

    // resolve regular function meta
    // TODO 这里这个`flattened_symbols_by_uri`的cache可以考虑也在前面trait document symbol的请求里使用
    let mut flattened_symbols_by_uri: HashMap<String, Vec<DocumentSymbolWithPath>> = HashMap::new();
    for (file_uri, line, character, extra) in regular_function_targets {
        let regular_meta = resolve_regular_function_target(
            client,
            &file_uri,
            line,
            character,
            &extra,
            &mut flattened_symbols_by_uri,
        )
        .await?;
        insert_function_meta(&mut function_metas_by_id, regular_meta);
    }

    if function_metas_by_id.is_empty() {
        // No trait impl methods were found and no regular function target resolved successfully.
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
    let target_function_names: HashSet<String> = function_metas_by_id
        .values()
        .map(|meta| meta.function_name.clone())
        .collect();

    let mut graph: DiGraph<String, ()> = DiGraph::new();
    let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();
    for id in function_metas_by_id.keys() {
        let index = graph.add_node(id.clone());
        node_indices.insert(id.clone(), index);
    }

    // Cache for `textDocument/implementation` during edge resolution (keyed by uri+position).
    let mut implementation_cache: HashMap<String, Vec<Location>> = HashMap::new();
    let mut parse_deps_context = ParseDepsContext {
        graph: &mut graph,
        node_indices: &node_indices,
        functions_by_uri: &functions_by_uri,
        target_function_names: &target_function_names,
        implementation_cache: &mut implementation_cache,
    };

    for meta in function_metas_by_id.values_mut() {
        parse_function_deps_by_outgoing_calls_recursive(
            client,
            meta,
            &mut HashSet::new(),
            None,
            &mut parse_deps_context,
        )
        .await?;
    }

    let mut output = Vec::with_capacity(function_metas_by_id.len());
    for meta in function_metas_by_id.values() {
        let node_index = node_indices
            .get(&meta.id)
            .ok_or_else(|| anyhow::anyhow!("graph node missing for {}", meta.id))?;
        let mut deps = Vec::new();

        for dep_node_idx in graph.neighbors_directed(*node_index, petgraph::Direction::Outgoing) {
            let dep_meta_id = graph
                .node_weight(dep_node_idx)
                .ok_or_else(|| anyhow::anyhow!("graph node missing for {:?}", dep_node_idx))?;
            let callee_meta = function_metas_by_id
                .get(dep_meta_id)
                .ok_or_else(|| anyhow::anyhow!("graph node missing for {:?}", dep_meta_id))?;
            deps.push(AnalyzeFuncDepsGraphDependency {
                fn_type: callee_meta.fn_type.clone(),
                file_uri: callee_meta.file_uri.clone(),
                function_name: callee_meta.function_name.clone(),
                range: callee_meta.range.clone(),
            });
        }

        deps.sort_by(|a, b| {
            a.fn_type
                .cmp(&b.fn_type)
                .then_with(|| a.file_uri.cmp(&b.file_uri))
                .then_with(|| a.function_name.cmp(&b.function_name))
        });
        output.push(AnalyzeFuncDepsGraphItem {
            fn_type: meta.fn_type.clone(),
            extra: meta.extra.clone(),
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
            .then_with(|| a.fn_type.cmp(&b.fn_type))
            .then_with(|| a.function_name.cmp(&b.function_name))
    });

    Ok(output)
}

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
/// This id is used internally as the dependency edge payload and for set membership checks.
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
/// 需要判断该callee item是否可能为目标函数集合中的函数，如果是，返回其id，以及返回其是否通过fallback impl解析得到
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
    target_function_names: &HashSet<String>,
    node_indices: &HashMap<String, NodeIndex>,
    implementation_cache: &mut HashMap<String, Vec<Location>>,
) -> anyhow::Result<Vec<String>> {
    if let Some(id) = match_function_id_for_call_hierarchy_item(item, functions_by_uri)
        && node_indices.contains_key(&id)
    {
        // 处于目标函数集中的函数，直接返回id
        return Ok(vec![id]);
    }

    // 此时可能遇到了trait方法(trait方法在outgoing-call分析时，无法定位到其具体的impl代码的位置，而是定位到其定义的位置)，此时需要额外地调用implement来确定该trait方法是否属于目标函数集合中.
    let item_detail = item.detail.as_deref().unwrap_or("");
    // 这里预计已经收集到的目标函数名称是不包含函数签名的准确的名称，
    // 但通过outgoing-call得到的item的名称没有这个保证
    // 因此是检查item.name与item.detail中是否有包含目标的函数名称
    if !target_function_names.iter().any(|target_func_name| {
        item.name.contains(target_func_name) || item_detail.contains(target_func_name)
    }) {
        // 发现了不处于目标函数集合中的trait方法，该trait方法的内部链路可能会依赖目标函数集合中的函数，
        // 但因为无法确定其具体实现trait代码的位置，此时选择在extra中额外保存该item的信息
        return Ok(Vec::new());
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
    user_extra: &HashMap<String, Value>,
    out: &mut HashMap<String, FunctionMeta>,
) {
    for child in children {
        if matches!(child.kind, SymbolKind::Function | SymbolKind::Method) {
            let id = function_id_from_range(document_uri, &child.range);
            insert_function_meta(
                out,
                FunctionMeta {
                    id,
                    fn_type: AnalyzeFuncDepsGraphFnType::TraitImpl,
                    extra: build_trait_impl_extra(trait_name, user_extra),
                    function_name: child.name.clone(),
                    file_uri: document_uri.to_string(),
                    range: child.range.clone(),
                    selection_range: child.selection_range.clone(),
                },
            );
        }

        if let Some(grandchildren) = &child.children {
            collect_impl_functions(trait_name, document_uri, grandchildren, user_extra, out);
        }
    }
}

fn insert_function_meta(out: &mut HashMap<String, FunctionMeta>, meta: FunctionMeta) {
    if let Some(existing) = out.get_mut(&meta.id) {
        // A single function can be selected by multiple targets. Instead of dropping later target
        // metadata, merge it so the final item preserves every user-supplied payload that
        // contributed to this node.
        merge_extra_metadata(&mut existing.extra, meta.extra);
        if existing.fn_type != AnalyzeFuncDepsGraphFnType::TraitImpl
            && meta.fn_type == AnalyzeFuncDepsGraphFnType::TraitImpl
        {
            existing.fn_type = meta.fn_type;
        }
        return;
    }
    out.insert(meta.id.clone(), meta);
}

async fn resolve_regular_function_target(
    client: &LSPClient,
    file_uri: &str,
    line: u32,
    character: u32,
    user_extra: &HashMap<String, Value>,
    flattened_symbols_by_uri: &mut HashMap<String, Vec<DocumentSymbolWithPath>>,
) -> anyhow::Result<FunctionMeta> {
    let flattened = if let Some(cached) = flattened_symbols_by_uri.get(file_uri) {
        cached.clone()
    } else {
        let document_symbols = client
            .document_symbols(file_uri.to_string())
            .await
            .map_err(|error| {
                anyhow::anyhow!(
                    "failed to inspect symbols for regular function target '{}': {}",
                    file_uri,
                    error
                )
            })?;
        let mut flattened = Vec::new();
        flatten_document_symbols(&document_symbols, &mut flattened);
        flattened_symbols_by_uri.insert(file_uri.to_string(), flattened.clone());
        flattened
    };

    let position = Position { line, character };
    let mut matches: Vec<DocumentSymbolWithPath> = flattened
        .into_iter()
        .filter(|node| {
            matches!(node.symbol.kind, SymbolKind::Function | SymbolKind::Method)
                && range_contains_position(&node.symbol.range, &position)
        })
        .collect();
    matches.sort_by(|a, b| {
        symbol_range_span(&a.symbol.range)
            .cmp(&symbol_range_span(&b.symbol.range))
            .then_with(|| a.symbol.name.cmp(&b.symbol.name))
    });

    let Some(symbol) = matches.into_iter().next() else {
        anyhow::bail!(
            "regular function target '{}:{}:{}' does not resolve to a function or method symbol",
            file_uri,
            line,
            character
        );
    };

    Ok(FunctionMeta {
        id: function_id_from_range(file_uri, &symbol.symbol.range),
        fn_type: AnalyzeFuncDepsGraphFnType::RegularFunction,
        extra: user_extra.clone(),
        function_name: symbol.symbol.name,
        file_uri: file_uri.to_string(),
        range: symbol.symbol.range,
        selection_range: symbol.symbol.selection_range,
    })
}

fn symbol_range_span(range: &Range) -> (u32, u32) {
    (
        range.end.line.saturating_sub(range.start.line),
        range.end.character.saturating_sub(range.start.character),
    )
}

fn build_trait_impl_extra(
    trait_name: &str,
    user_extra: &HashMap<String, Value>,
) -> HashMap<String, Value> {
    let mut extra = HashMap::new();
    extra.insert(
        "trait_name".to_string(),
        Value::String(trait_name.to_string()),
    );
    for (key, value) in user_extra {
        extra.insert(key.clone(), value.clone());
    }
    extra
}

fn merge_extra_metadata(existing: &mut HashMap<String, Value>, incoming: HashMap<String, Value>) {
    existing.extend(incoming);
}

fn parse_file_uri(file_uri: &str) -> anyhow::Result<Uri<&str>> {
    let trimmed = file_uri.trim();
    if trimmed.is_empty() {
        anyhow::bail!("function target file URI must not be empty");
    }
    let parsed = Uri::parse(trimmed)
        .map_err(|error| anyhow::anyhow!("invalid URI '{}': {}", trimmed, error))?;
    if parsed.scheme().as_str() != "file" {
        anyhow::bail!("function target must use a file:// URI, got '{}'", trimmed);
    }
    let path = PathBuf::from(parsed.path().as_str());
    if !path.exists() {
        anyhow::bail!("function target file '{}' does not exist", path.display());
    }
    if !path.is_file() {
        anyhow::bail!(
            "function target '{}' must point to a file, not a directory",
            path.display()
        );
    }
    Ok(parsed)
}

/// bfs遍历目标函数的所有外部调用，并建立依赖关系
/// This function intentionally follows the full reachable outgoing call chain rather than
/// only the first hop. That behavior is required so a selected function is marked as
/// depending on another selected function even when the connection is indirect.
async fn parse_function_deps_by_outgoing_calls_recursive(
    client: &LSPClient,
    meta: &mut FunctionMeta,
    visited: &mut HashSet<String>,
    max_depth: Option<usize>,
    context: &mut ParseDepsContext<'_>,
) -> anyhow::Result<()> {
    let project_root = client.project_root.to_string_lossy();
    let caller_index = context
        .node_indices
        .get(&meta.id)
        .copied()
        .ok_or_else(|| anyhow::anyhow!("graph node missing for {}", meta.id))?;

    let items = client
        .prepare_call_hierarchy(
            meta.file_uri.clone(),
            meta.selection_range.start.line,
            meta.selection_range.start.character,
        )
        .await?;

    let mut queue = VecDeque::new();
    for item in items {
        let key = format!(
            "{}:{}:{}",
            item.uri, item.range.start.line, item.range.start.character
        );
        // 防止递归调用，这里先标记自己
        visited.insert(key.clone());

        queue.push_back((item, 0));
    }

    while let Some((item, depth)) = queue.pop_front() {
        if let Some(max) = max_depth
            && depth >= max
        {
            continue;
        }

        let outgoing_calls = client.outgoing_calls(item.clone()).await?;

        for call in outgoing_calls {
            let key = format!(
                "{}:{}:{}",
                call.to.uri, call.to.range.start.line, call.to.range.start.character
            );
            if visited.contains(&key) {
                continue;
            }
            visited.insert(key.clone());

            // 如果被调用的item不属于当前项目，那么直接跳过
            let callee_file_uri = parse_file_uri(&call.to.uri)?;
            if !callee_file_uri
                .path()
                .as_str()
                .starts_with(project_root.as_ref())
            {
                continue;
            }

            // Map the callee `CallHierarchyItem` to one or more target function ids.
            //
            // The common case resolves via direct range matching. The fallback performs an
            // implementation lookup to bridge trait-method calls back to concrete impl methods.
            let callee_ids = resolve_target_function_ids_for_call(
                client,
                &call.to,
                context.functions_by_uri,
                context.target_function_names,
                context.node_indices,
                context.implementation_cache,
            )
            .await?;

            // 遇到了一个不属于目标函数集合中的函数，需要额外处理
            if callee_ids.is_empty() {
                // 额外在extra中标记遇到了处于当前项目内，但是没有解析的trait函数
                let key = format!(
                    "{}:{}:{}",
                    call.to.uri,
                    call.to.selection_range.start.line,
                    call.to.selection_range.start.character
                );
                // 通过判断实现该函数的impl代码是否存在来判断它是不是trait方法
                let is_trait_method = if let Some(locations) = context.implementation_cache.get(&key)
                {
                    !locations.is_empty()
                } else {
                    let fetched = client
                        .implementation(
                            call.to.uri.clone(),
                            call.to.selection_range.start.line,
                            call.to.selection_range.start.character,
                        )
                        .await?;
                    context.implementation_cache.insert(key, fetched.clone());

                    !fetched.is_empty()
                };

                if is_trait_method {
                    let unknown_dep_trait_functions = meta
                        .extra
                        .entry("unknown_dep_trait_functions".to_string())
                        .or_insert_with(|| Value::Array(vec![]));
                    let function_id = function_id_from_range(&call.to.uri, &call.to.range);
                    unknown_dep_trait_functions
                        .as_array_mut()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "failed to convert the `unknown_dep_trait_functions` field to array"
                            )
                        })?
                        .push(serde_json::json!({
                            "function_id": function_id,
                        }));
                }

                // 遇到了目标函数集合外的函数，将其队列继续分析其更深的调用关系
                // 而遇到目标函数时不入队，避免重复分析
                queue.push_back((call.to, depth + 1));
                continue;
            }

            for callee_id in callee_ids {
                if callee_id == meta.id {
                    continue;
                }
                let Some(&callee_index) = context.node_indices.get(&callee_id) else {
                    continue;
                };
                context.graph.update_edge(caller_index, callee_index, ());
            }
        }
    }

    Ok(())
}

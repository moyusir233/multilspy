use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use multilspy_rust::config::RustAnalyzerConfig;
use multilspy_rust::logic::RecursiveCallHierarchy;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::position_utils::*;
use crate::ipc::types::*;

struct LSPInstance {
    lsp: Arc<Mutex<RecursiveCallHierarchy>>,
}

struct LSPManager {
    instances: Mutex<HashMap<String, Arc<LSPInstance>>>,
}

impl LSPManager {
    fn new() -> Self {
        Self {
            instances: Mutex::new(HashMap::new()),
        }
    }

    async fn get_or_create_instance(&self, project_path: String) -> Result<(), String> {
        // First check if instance already exists
        {
            let instances = self.instances.lock().await;
            if instances.contains_key(&project_path) {
                return Ok(());
            }
        }

        // Validate project path and create config (not holding lock)
        let project_root = PathBuf::from(&project_path);
        if !project_root.is_dir() {
            return Err(format!("Project path does not exist: {}", project_path));
        }

        let config = RustAnalyzerConfig::new(project_root);
        let mut lsp = RecursiveCallHierarchy::new(config);
        lsp.start().await.map_err(|e| format!("Failed to start LSP: {}", e))?;

        // Now insert, checking again in case someone else inserted while we were starting LSP
        let mut instances = self.instances.lock().await;
        if !instances.contains_key(&project_path) {
            instances.insert(project_path, Arc::new(LSPInstance { lsp: Arc::new(Mutex::new(lsp)) }));
        }

        Ok(())
    }

    async fn get_instance(&self, project_path: &str) -> Option<Arc<LSPInstance>> {
        let instances = self.instances.lock().await;
        instances.get(project_path).cloned()
    }

    async fn take_instance(&self, project_path: &str) -> Option<Arc<LSPInstance>> {
        let mut instances = self.instances.lock().await;
        instances.remove(project_path)
    }
}

#[derive(Clone)]
struct AppState {
    manager: Arc<LSPManager>,
}

pub async fn start_server() {
    let manager = Arc::new(LSPManager::new());
    let state = AppState { manager };

    let app = Router::new()
        .route("/health", get(health))
        .route("/start", post(start))
        .route("/shutdown", post(shutdown))
        .route("/status", get(status))
        .route("/definition", post(definition))
        .route("/references", post(references))
        .route("/document_symbols", post(document_symbols))
        .route("/incoming_calls", post(incoming_calls))
        .route("/outgoing_calls", post(outgoing_calls))
        .route("/incoming_calls_recursive", post(incoming_calls_recursive))
        .route("/outgoing_calls_recursive", post(outgoing_calls_recursive))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<ApiResponse> {
    Json(ApiResponse::ok(()))
}

#[derive(Serialize)]
struct StartResult {
    project_path: String,
}

async fn start(
    State(state): State<AppState>,
    Json(req): Json<StartRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() {
        return Json(ApiResponse::error("project_path is required".to_string()));
    }

    match state.manager.get_or_create_instance(req.project_path.clone()).await {
        Ok(_) => Json(ApiResponse::ok(StartResult { project_path: req.project_path })),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

async fn shutdown(
    State(state): State<AppState>,
    Json(req): Json<StartRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() {
        return Json(ApiResponse::error("project_path is required".to_string()));
    }

    // Take the instance out of the map first (without holding lock across await)
    let maybe_instance = state.manager.take_instance(&req.project_path).await;

    if let Some(instance) = maybe_instance {
        // Now we have ownership of the instance, stop it
        let mut lsp = instance.lsp.lock().await;
        let _ = lsp.stop().await;
    }

    Json(ApiResponse::stopped())
}

async fn status(
    State(state): State<AppState>,
) -> Json<ApiResponse> {
    let instances = state.manager.instances.lock().await;
    let statuses: Vec<_> = instances.keys().cloned().collect();
    Json(ApiResponse::ok(statuses))
}

async fn definition(
    State(state): State<AppState>,
    Json(req): Json<PositionRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);
    let lsp_pos = raw_to_lsp_position(req.line, req.column);

    let result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.client.definition(file_uri, lsp_pos.line, lsp_pos.character).await
    };

    match result {
        Ok(locations) => {
            let raw_locations = convert_all_locations_to_raw(&locations);
            Json(ApiResponse::ok(raw_locations))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to get definition: {}", e))),
    }
}

async fn references(
    State(state): State<AppState>,
    Json(req): Json<ReferencesRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);
    let lsp_pos = raw_to_lsp_position(req.line, req.column);

    let result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.client.references(file_uri, lsp_pos.line, lsp_pos.character, true).await
    };

    match result {
        Ok(locations) => {
            let raw_locations = convert_all_locations_to_raw(&locations);
            Json(ApiResponse::ok(raw_locations))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to get references: {}", e))),
    }
}

async fn document_symbols(
    State(state): State<AppState>,
    Json(req): Json<DocumentSymbolsRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);

    let result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.client.document_symbols(file_uri).await
    };

    match result {
        Ok(symbols) => {
            let raw_symbols = convert_document_symbols_to_raw(&symbols);
            Json(ApiResponse::ok(raw_symbols))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to get document symbols: {}", e))),
    }
}

async fn incoming_calls(
    State(state): State<AppState>,
    Json(req): Json<PositionRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);
    let lsp_pos = raw_to_lsp_position(req.line, req.column);

    let items_result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.client.prepare_call_hierarchy(file_uri.clone(), lsp_pos.line, lsp_pos.character).await
    };

    let items = match items_result {
        Ok(items) => items,
        Err(e) => return Json(ApiResponse::error(format!("Failed to prepare call hierarchy: {}", e))),
    };

    let mut all_calls = Vec::new();
    for item in items {
        let calls_result = {
            let mut lsp_guard = lsp.lock().await;
            lsp_guard.client.incoming_calls(item).await
        };
        match calls_result {
            Ok(calls) => all_calls.extend(calls),
            Err(e) => return Json(ApiResponse::error(format!("Failed to get incoming calls: {}", e))),
        }
    }

    let raw_calls = convert_incoming_calls_to_raw(&all_calls);
    Json(ApiResponse::ok(raw_calls))
}

async fn outgoing_calls(
    State(state): State<AppState>,
    Json(req): Json<PositionRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);
    let lsp_pos = raw_to_lsp_position(req.line, req.column);

    let items_result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.client.prepare_call_hierarchy(file_uri.clone(), lsp_pos.line, lsp_pos.character).await
    };

    let items = match items_result {
        Ok(items) => items,
        Err(e) => return Json(ApiResponse::error(format!("Failed to prepare call hierarchy: {}", e))),
    };

    let mut all_calls = Vec::new();
    for item in items {
        let calls_result = {
            let mut lsp_guard = lsp.lock().await;
            lsp_guard.client.outgoing_calls(item).await
        };
        match calls_result {
            Ok(calls) => all_calls.extend(calls),
            Err(e) => return Json(ApiResponse::error(format!("Failed to get outgoing calls: {}", e))),
        }
    }

    let raw_calls = convert_outgoing_calls_to_raw(&all_calls);
    Json(ApiResponse::ok(raw_calls))
}

async fn incoming_calls_recursive(
    State(state): State<AppState>,
    Json(req): Json<RecursiveRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);
    let lsp_pos = raw_to_lsp_position(req.line, req.column);

    let result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.clear_visited();
        lsp_guard.incoming_calls_recursive(file_uri, lsp_pos.line, lsp_pos.character, req.max_depth).await
    };

    match result {
        Ok(results) => {
            let mut result_map = RecursiveIncomingCallsResult::new();

            for (item, calls) in results {
                let key = get_call_hierarchy_key(&item);
                let info = extract_call_hierarchy_item_info(&item);

                let mut incoming_refs = Vec::new();
                for call in calls {
                    let from_key = get_call_hierarchy_key(&call.from);
                    let from_ranges = call.from_ranges.iter().map(convert_lsp_range_to_raw).collect();
                    incoming_refs.push(RecursiveIncomingCallRef {
                        key: from_key,
                        from_ranges,
                    });
                }

                result_map.insert(key, RecursiveIncomingCallEntry {
                    info,
                    incoming_calls: incoming_refs,
                });
            }

            Json(ApiResponse::ok(result_map))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to get recursive incoming calls: {}", e))),
    }
}

async fn outgoing_calls_recursive(
    State(state): State<AppState>,
    Json(req): Json<RecursiveRequest>,
) -> Json<ApiResponse> {
    if req.project_path.is_empty() || req.file_path.is_empty() {
        return Json(ApiResponse::error("project_path and file_path are required".to_string()));
    }

    let Some(instance) = state.manager.get_instance(&req.project_path).await else {
        return Json(ApiResponse::error("Instance not found for project path".to_string()));
    };

    let lsp = instance.lsp.clone();
    let file_uri = format!("file://{}", req.file_path);
    let lsp_pos = raw_to_lsp_position(req.line, req.column);

    let result = {
        let mut lsp_guard = lsp.lock().await;
        lsp_guard.clear_visited();
        lsp_guard.outgoing_calls_recursive(file_uri, lsp_pos.line, lsp_pos.character, req.max_depth).await
    };

    match result {
        Ok(results) => {
            let mut result_map = RecursiveOutgoingCallsResult::new();

            for (item, calls) in results {
                let key = get_call_hierarchy_key(&item);
                let info = extract_call_hierarchy_item_info(&item);

                let mut outgoing_refs = Vec::new();
                for call in calls {
                    let to_key = get_call_hierarchy_key(&call.to);
                    let from_ranges = call.from_ranges.iter().map(convert_lsp_range_to_raw).collect();
                    outgoing_refs.push(RecursiveOutgoingCallRef {
                        key: to_key,
                        from_ranges,
                    });
                }

                result_map.insert(key, RecursiveOutgoingCallEntry {
                    info,
                    outgoing_calls: outgoing_refs,
                });
            }

            Json(ApiResponse::ok(result_map))
        }
        Err(e) => Json(ApiResponse::error(format!("Failed to get recursive outgoing calls: {}", e))),
    }
}

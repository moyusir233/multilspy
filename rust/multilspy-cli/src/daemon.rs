use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use multilspy_rust::{LSPClient, RustAnalyzerConfig};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use crate::ipc::*;
use crate::lifecycle;

const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(7200);

struct DaemonState {
    client: LSPClient,
    workspace: PathBuf,
    started_at: Instant,
    last_activity: Mutex<Instant>,
    port: u16,
}

pub async fn run_daemon(
    workspace: PathBuf,
    initialize_params_path: PathBuf,
    wait_work_done_progress_create_max_time_secs: Option<u64>,
) -> anyhow::Result<()> {
    let config = match wait_work_done_progress_create_max_time_secs {
        Some(secs) => RustAnalyzerConfig::new(workspace.clone(), initialize_params_path)
            .with_wait_work_done_progress_create_max_time(Duration::from_secs(secs)),
        None => RustAnalyzerConfig::new(workspace.clone(), initialize_params_path),
    };

    let canonical = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let pid = std::process::id();
    lifecycle::write_pidfile(&canonical, pid, port)?;
    tracing::info!("daemon: listening on 127.0.0.1:{}, pid={}", port, pid);

    tracing::info!("daemon: initializing LSPClient for {:?}", workspace);
    let client = LSPClient::new(config).await?;
    tracing::info!("daemon: LSPClient ready");

    let now = Instant::now();
    let state = Arc::new(DaemonState {
        client,
        workspace: canonical.clone(),
        started_at: now,
        last_activity: Mutex::new(now),
        port,
    });

    let shutdown_notify = Arc::new(tokio::sync::Notify::new());

    let state_for_timeout = state.clone();
    let shutdown_for_timeout = shutdown_notify.clone();
    tokio::spawn(async move {
        loop {
            // Check inactivity timeout every 30 seconds
            // if last activity is older than INACTIVITY_TIMEOUT, shutdown
            tokio::time::sleep(Duration::from_secs(30)).await;
            let last = *state_for_timeout.last_activity.lock().await;
            if last.elapsed() >= INACTIVITY_TIMEOUT {
                tracing::info!("daemon: inactivity timeout reached, shutting down");
                shutdown_for_timeout.notify_waiters();
                return;
            }
        }
    });

    let app = Router::new()
        .route("/rpc", post(rpc_handler))
        .with_state(state.clone());

    let shutdown_for_serve = shutdown_notify.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_for_serve.notified().await;
        })
        .await?;

    let _ = lifecycle::remove_pidfile(&state.workspace);
    state.client.clone().shutdown().await?;
    tracing::info!("daemon: shutdown complete");

    Ok(())
}

async fn rpc_handler(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<IpcRequest>,
) -> Json<IpcResponse> {
    {
        *state.last_activity.lock().await = Instant::now();
    }
    Json(dispatch(req, &state).await)
}

async fn dispatch(req: IpcRequest, state: &DaemonState) -> IpcResponse {
    match req.method.as_str() {
        "ping" => IpcResponse::success(serde_json::json!("pong")),

        "shutdown" => {
            let _ = lifecycle::remove_pidfile(&state.workspace);
            tokio::spawn({
                let client = state.client.clone();
                async move {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    let _ = client.shutdown().await;
                    std::process::exit(0);
                }
            });
            IpcResponse::success(serde_json::json!("shutdown_ack"))
        }

        "status" => {
            let resp = StatusResponse {
                workspace: state.workspace.display().to_string(),
                pid: std::process::id(),
                port: state.port,
                uptime_secs: state.started_at.elapsed().as_secs(),
            };
            match serde_json::to_value(resp) {
                Ok(v) => IpcResponse::success(v),
                Err(e) => IpcResponse::error(ERR_INTERNAL, e.to_string()),
            }
        }

        "definition" => {
            let params: PositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state
                .client
                .definition(params.uri, params.line, params.character)
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "type-definition" => {
            let params: PositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state
                .client
                .type_definition(params.uri, params.line, params.character)
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "implementation" => {
            let params: PositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state
                .client
                .implementation(params.uri, params.line, params.character)
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "references" => {
            let params: ReferencesIpcParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state
                .client
                .references(
                    params.uri,
                    params.line,
                    params.character,
                    params.include_declaration,
                )
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "document-symbols" => {
            let params: DocumentSymbolsIpcParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state.client.document_symbols(params.uri).await {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "workspace-symbols" => {
            let params: WorkspaceSymbolsIpcParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state.client.workspace_symbols(params.query).await {
                Ok(mut r) => {
                    if let Some(limit) = params.limit {
                        r.truncate(limit);
                    }
                    to_success(r)
                }
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "workspace-symbol-resolve" => {
            let params: WorkspaceSymbolResolveIpcParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state.client.workspace_symbol_resolve(params.symbol).await {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "incoming-calls" => {
            let params: PositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            let items = match state
                .client
                .prepare_call_hierarchy(params.uri, params.line, params.character)
                .await
            {
                Ok(items) => items,
                Err(e) => return IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            };
            let mut all_calls = Vec::new();
            for item in items {
                match state.client.incoming_calls(item).await {
                    Ok(calls) => all_calls.extend(calls),
                    Err(e) => return IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
                }
            }
            to_success(all_calls)
        }

        "outgoing-calls" => {
            let params: PositionParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            let items = match state
                .client
                .prepare_call_hierarchy(params.uri, params.line, params.character)
                .await
            {
                Ok(items) => items,
                Err(e) => return IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            };
            let mut all_calls = Vec::new();
            for item in items {
                match state.client.outgoing_calls(item).await {
                    Ok(calls) => all_calls.extend(calls),
                    Err(e) => return IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
                }
            }
            to_success(all_calls)
        }

        "incoming-calls-recursive" => {
            let params: RecursiveCallsIpcParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state
                .client
                .incoming_calls_recursive(
                    params.uri,
                    params.line,
                    params.character,
                    params.max_depth,
                )
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "outgoing-calls-recursive" => {
            let params: RecursiveCallsIpcParams = match serde_json::from_value(req.params) {
                Ok(p) => p,
                Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
            };
            match state
                .client
                .outgoing_calls_recursive(
                    params.uri,
                    params.line,
                    params.character,
                    params.max_depth,
                )
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        "analyze-trait-impl-deps-graph" => {
            let params: AnalyzeTraitImplDepsGraphIpcParams =
                match serde_json::from_value(req.params) {
                    Ok(p) => p,
                    Err(e) => return IpcResponse::error(ERR_INVALID_PARAMS, e.to_string()),
                };
            match state
                .client
                .analyze_trait_impl_deps_graph(params.trait_names, params.target_dir_uris)
                .await
            {
                Ok(r) => to_success(r),
                Err(e) => IpcResponse::error(ERR_LSP_FAILED, e.to_string()),
            }
        }

        other => IpcResponse::error(ERR_METHOD_NOT_FOUND, format!("unknown method: {}", other)),
    }
}

fn to_success<T: serde::Serialize>(value: T) -> IpcResponse {
    match serde_json::to_value(value) {
        Ok(v) => IpcResponse::success(v),
        Err(e) => IpcResponse::error(ERR_INTERNAL, e.to_string()),
    }
}

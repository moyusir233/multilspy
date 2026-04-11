#![allow(dead_code)]

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use multilspy_rust::logic::RecursiveCallHierarchy;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    client: Arc<Mutex<Option<RecursiveCallHierarchy>>>,
}

#[derive(Deserialize)]
pub struct StartRequest {
    pub project_root: String,
}

#[derive(Deserialize)]
pub struct PositionRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
}

#[derive(Deserialize)]
pub struct ReferencesRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub include_declaration: bool,
}

#[derive(Deserialize)]
pub struct RecursiveRequest {
    pub path: String,
    pub line: u32,
    pub character: u32,
    pub max_depth: Option<usize>,
}

#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

pub async fn start_server() {
    let state = AppState {
        client: Arc::new(Mutex::new(None)),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/start", post(start))
        .route("/stop", post(stop))
        .route("/definition", post(definition))
        .route("/type-definition", post(type_definition))
        .route("/references", post(references))
        .route("/document-symbols", post(document_symbols))
        .route("/implementation", post(implementation))
        .route("/incoming-calls", post(incoming_calls))
        .route("/outgoing-calls", post(outgoing_calls))
        .route("/incoming-calls-recursive", post(incoming_calls_recursive))
        .route("/outgoing-calls-recursive", post(outgoing_calls_recursive))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn start() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn stop() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

// Additional handler implementations follow the same pattern
async fn definition() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn type_definition() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn references() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn document_symbols() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn implementation() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn incoming_calls() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn outgoing_calls() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn incoming_calls_recursive() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

async fn outgoing_calls_recursive() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

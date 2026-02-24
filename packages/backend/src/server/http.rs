use crate::session::manager::SessionManager;
use crate::server::ws::handle_ws;
use crate::types::{SearchScope, ServerEvent};
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

pub struct AppState {
    pub session_manager: Arc<SessionManager>,
    pub broadcast_tx: broadcast::Sender<ServerEvent>,
    pub message_tx: broadcast::Sender<ServerEvent>,
}

pub fn create_router(state: Arc<AppState>, frontend_dist: Option<String>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/sessions", get(sessions_handler))
        .route("/api/sessions/{session_id}", get(session_detail_handler))
        .route("/api/search", get(search_handler))
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(state.clone());

    if let Some(dist_path) = frontend_dist {
        // Serve static files from the frontend build, with SPA fallback
        let serve_dir = ServeDir::new(&dist_path);
        let fallback_path = dist_path.clone();

        api.fallback_service(
            serve_dir.fallback(get(move || {
                let path = fallback_path.clone();
                async move {
                    match tokio::fs::read_to_string(format!("{}/index.html", path)).await {
                        Ok(html) => Html(html).into_response(),
                        Err(_) => StatusCode::NOT_FOUND.into_response(),
                    }
                }
            })),
        )
    } else {
        api
    }
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn sessions_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<crate::types::AgentSessionSummary>> {
    let sessions = state.session_manager.get_sessions().await;
    Json(sessions)
}

async fn session_detail_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> Response {
    match state.session_manager.get_session_detail(&session_id).await {
        Some(detail) => Json(detail).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Session not found" })),
        )
            .into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let session_manager = state.session_manager.clone();
    let broadcast_rx = state.broadcast_tx.subscribe();
    let message_rx = state.message_tx.subscribe();

    ws.on_upgrade(move |socket| {
        handle_ws(socket, session_manager, broadcast_rx, message_rx)
    })
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    scope: Option<String>,
}

fn parse_scopes(scope_str: &str) -> Vec<SearchScope> {
    scope_str
        .split(',')
        .filter_map(|s| match s.trim() {
            "project_name" => Some(SearchScope::ProjectName),
            "current_task" => Some(SearchScope::CurrentTask),
            "working_directory" => Some(SearchScope::WorkingDirectory),
            "content" => Some(SearchScope::Content),
            _ => None,
        })
        .collect()
}

const ALL_SCOPES: [SearchScope; 4] = [
    SearchScope::ProjectName,
    SearchScope::CurrentTask,
    SearchScope::WorkingDirectory,
    SearchScope::Content,
];

async fn search_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQuery>,
) -> Response {
    if params.q.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Query parameter 'q' is required" })),
        )
            .into_response();
    }

    let scopes = match &params.scope {
        Some(s) if !s.is_empty() => {
            let parsed = parse_scopes(s);
            if parsed.is_empty() {
                ALL_SCOPES.to_vec()
            } else {
                parsed
            }
        }
        _ => ALL_SCOPES.to_vec(),
    };

    let response = state
        .session_manager
        .search_sessions(&params.q, &scopes)
        .await;
    Json(response).into_response()
}

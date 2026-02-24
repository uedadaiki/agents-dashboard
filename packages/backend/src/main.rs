mod cost;
mod providers;
mod server;
mod session;
mod types;

use providers::ProviderEvent;
use server::http::{create_router, AppState};
use session::manager::SessionManager;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::info;
use types::ServerEvent;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agents_dashboard_backend=info".into()),
        )
        .init();

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    // Broadcast channels
    let (broadcast_tx, _) = broadcast::channel::<ServerEvent>(256);
    let (message_tx, _) = broadcast::channel::<ServerEvent>(1024);

    // Session manager
    let session_manager = Arc::new(SessionManager::new());
    session_manager.start().await;

    // Frontend dist path
    let frontend_dist = ["packages/frontend/build", "../frontend/build"]
        .iter()
        .map(std::path::PathBuf::from)
        .find(|p| p.join("index.html").exists())
        .map(|p| p.to_string_lossy().to_string());

    if let Some(ref dist) = frontend_dist {
        info!("Serving frontend from {}", dist);
    } else {
        info!("No frontend build found, serving API only");
    }

    let state = Arc::new(AppState {
        session_manager: session_manager.clone(),
        broadcast_tx: broadcast_tx.clone(),
        message_tx: message_tx.clone(),
    });

    let app = create_router(state, frontend_dist);

    // Route provider events to broadcast channels
    let broadcast_tx_route = broadcast_tx.clone();
    let message_tx_route = message_tx.clone();
    let sm_route = session_manager.clone();
    tokio::spawn(async move {
        let mut event_rx = sm_route.event_rx.lock().await;
        while let Some(event) = event_rx.recv().await {
            let server_event = match &event {
                ProviderEvent::SessionDiscovered { session } => {
                    info!(
                        "[Session] Discovered: {} ({})",
                        session.session_id, session.project_name
                    );
                    Some(ServerEvent::SessionDiscovered {
                        session: session.clone(),
                    })
                }
                ProviderEvent::SessionRemoved { session_id } => {
                    info!("[Session] Removed: {}", session_id);
                    Some(ServerEvent::SessionRemoved {
                        session_id: session_id.clone(),
                    })
                }
                ProviderEvent::StateChanged {
                    session_id,
                    previous,
                    current,
                } => {
                    info!("[Session] {}: {} → {}", session_id, previous, current);
                    if let Some(session) = sm_route.get_session_summary(session_id).await {
                        Some(ServerEvent::StateChanged {
                            session_id: session_id.clone(),
                            previous: *previous,
                            current: *current,
                            session,
                        })
                    } else {
                        None
                    }
                }
                ProviderEvent::UsageUpdated { session_id, usage } => {
                    Some(ServerEvent::UsageUpdated {
                        session_id: session_id.clone(),
                        usage: usage.clone(),
                    })
                }
                ProviderEvent::NewMessage {
                    session_id,
                    message,
                } => {
                    let msg_event = ServerEvent::NewMessage {
                        session_id: session_id.clone(),
                        message: message.clone(),
                    };
                    let _ = message_tx_route.send(msg_event);
                    None
                }
                ProviderEvent::GitStatusUpdated {
                    session_id,
                    git_status,
                } => Some(ServerEvent::GitStatusUpdated {
                    session_id: session_id.clone(),
                    git_status: git_status.clone(),
                }),
            };

            if let Some(evt) = server_event {
                let _ = broadcast_tx_route.send(evt);
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind");

    info!(
        "[Server] Agents Dashboard backend running on http://localhost:{}",
        port
    );

    // Graceful shutdown
    let sm_shutdown = session_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("\n[Server] Shutting down...");
        sm_shutdown.stop().await;
        std::process::exit(0);
    });

    axum::serve(listener, app).await.expect("Server error");
}

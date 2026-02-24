use crate::session::manager::SessionManager;
use crate::types::{ClientEvent, ServerEvent};
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::debug;

/// Handle a single WebSocket connection.
pub async fn handle_ws(
    socket: WebSocket,
    session_manager: Arc<SessionManager>,
    broadcast_rx: broadcast::Receiver<ServerEvent>,
    message_rx: broadcast::Receiver<ServerEvent>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let mut subscribed: HashSet<String> = HashSet::new();

    // Send initial sessions list
    let sessions = session_manager.get_sessions().await;
    let init_event = ServerEvent::SessionsInit { sessions };
    if let Ok(json) = serde_json::to_string(&init_event) {
        if ws_tx.send(Message::Text(json.into())).await.is_err() {
            return;
        }
    }

    // Channel for messages to send to this client
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<String>();

    // Task: forward broadcast events to client
    let client_tx_broadcast = client_tx.clone();
    let mut broadcast_rx = broadcast_rx;
    tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(event) => {
                    if let Ok(json) = serde_json::to_string(&event) {
                        if client_tx_broadcast.send(json).is_err() {
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    debug!("Broadcast lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    // Task: forward message events (filtered by subscription) to client
    let client_tx_message = client_tx.clone();
    let mut message_rx = message_rx;
    let (sub_update_tx, mut sub_update_rx) = mpsc::unbounded_channel::<SubUpdate>();

    tokio::spawn(async move {
        let mut local_subs: HashSet<String> = HashSet::new();

        loop {
            tokio::select! {
                update = sub_update_rx.recv() => {
                    match update {
                        Some(SubUpdate::Add(id)) => { local_subs.insert(id); }
                        Some(SubUpdate::Remove(id)) => { local_subs.remove(&id); }
                        None => break,
                    }
                }
                result = message_rx.recv() => {
                    match result {
                        Ok(event) => {
                            // Check if this message is for a subscribed session
                            let session_id = match &event {
                                ServerEvent::NewMessage { session_id, .. } => Some(session_id.as_str()),
                                _ => None,
                            };
                            if let Some(sid) = session_id {
                                if local_subs.contains(sid) {
                                    if let Ok(json) = serde_json::to_string(&event) {
                                        if client_tx_message.send(json).is_err() {
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            debug!("Message broadcast lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });

    // Main loop: read from client + write from client_rx
    loop {
        tokio::select! {
            // Outgoing messages
            Some(msg) = client_rx.recv() => {
                if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
            // Incoming messages from client
            result = ws_rx.next() => {
                match result {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(event) = serde_json::from_str::<ClientEvent>(&text) {
                            match event {
                                ClientEvent::Subscribe { session_id } => {
                                    subscribed.insert(session_id.clone());
                                    let _ = sub_update_tx.send(SubUpdate::Add(session_id.clone()));

                                    // Send existing messages for this session
                                    if let Some(messages) = session_manager.get_session_messages(&session_id).await {
                                        if !messages.is_empty() {
                                            let init = ServerEvent::MessagesInit {
                                                session_id: session_id.clone(),
                                                messages,
                                            };
                                            if let Ok(json) = serde_json::to_string(&init) {
                                                let _ = client_tx.send(json);
                                            }
                                        }
                                    }
                                }
                                ClientEvent::Unsubscribe { session_id } => {
                                    subscribed.remove(&session_id);
                                    let _ = sub_update_tx.send(SubUpdate::Remove(session_id));
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    debug!("WebSocket connection closed");
}

enum SubUpdate {
    Add(String),
    Remove(String),
}

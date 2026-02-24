use crate::providers::claude_code::ClaudeCodeProvider;
use crate::providers::ProviderEvent;
use crate::types::{AgentMessage, AgentSessionDetail, AgentSessionSummary, SearchResponse, SearchScope, SessionSearchResult};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

/// SessionManager wraps the provider and exposes an event channel.
pub struct SessionManager {
    provider: Arc<ClaudeCodeProvider>,
    pub event_rx: tokio::sync::Mutex<mpsc::UnboundedReceiver<ProviderEvent>>,
}

impl SessionManager {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let provider = Arc::new(ClaudeCodeProvider::new(event_tx));

        Self {
            provider,
            event_rx: tokio::sync::Mutex::new(event_rx),
        }
    }

    pub async fn start(&self) {
        self.provider.start().await;
        info!("[SessionManager] Started");
    }

    pub async fn stop(&self) {
        self.provider.stop().await;
        info!("[SessionManager] Stopped");
    }

    pub async fn get_sessions(&self) -> Vec<AgentSessionSummary> {
        self.provider.get_sessions().await
    }

    pub async fn get_session_detail(&self, session_id: &str) -> Option<AgentSessionDetail> {
        self.provider.get_session_detail(session_id).await
    }

    pub async fn get_session_messages(&self, session_id: &str) -> Option<Vec<AgentMessage>> {
        self.provider.get_session_messages(session_id).await
    }

    pub async fn get_session_summary(&self, session_id: &str) -> Option<AgentSessionSummary> {
        let sessions = self.provider.get_sessions().await;
        sessions.into_iter().find(|s| s.session_id == session_id)
    }

    pub async fn search_sessions(&self, query: &str, scopes: &[SearchScope]) -> SearchResponse {
        let results: Vec<SessionSearchResult> = self.provider.search_sessions(query, scopes).await;
        let total_sessions = results.len() as u32;
        SearchResponse {
            query: query.to_string(),
            total_sessions,
            results,
        }
    }
}

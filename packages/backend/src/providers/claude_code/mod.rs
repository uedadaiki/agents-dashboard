#[allow(dead_code)]
pub mod file_watcher;
#[allow(dead_code)]
pub mod jsonl_parser;
#[allow(dead_code)]
pub mod message_mapper;
#[allow(dead_code)]
pub mod session_discovery;
#[allow(dead_code)]
pub mod state_machine;

use crate::cost::add_usage;
use crate::providers::ProviderEvent;
use crate::types::{
    AgentMessage, AgentSessionDetail, AgentSessionSummary, AgentStateType, CumulativeUsage,
    GitStatus, MessageRole, MessageType, SearchMatch, SearchScope, SessionSearchResult,
};
use file_watcher::FileWatcher;
use jsonl_parser::RawEntry;
use message_mapper::{extract_model, extract_session_metadata, extract_usage, map_entry};
use session_discovery::{DiscoveredSession, DiscoveryEvent, SessionDiscovery};
use state_machine::{check_time_based_transitions, process_entry, StateContext};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

struct TrackedSession {
    summary: AgentSessionSummary,
    messages: Vec<AgentMessage>,
    state_ctx: StateContext,
    watcher: FileWatcher,
    model: String,
    emitted: bool,
    last_git_diff_check: i64,
    /// The project path from session discovery (decoded from directory name).
    /// Unlike summary.project_path which gets updated from JSONL cwd,
    /// this stays stable and is used to group sessions by project.
    discovery_project_path: String,
}

pub struct ClaudeCodeProvider {
    sessions: Arc<RwLock<HashMap<String, TrackedSession>>>,
    event_tx: mpsc::UnboundedSender<ProviderEvent>,
    shutdown: tokio::sync::watch::Sender<bool>,
    discovery: tokio::sync::Mutex<Option<SessionDiscovery>>,
}

impl ClaudeCodeProvider {
    pub fn new(event_tx: mpsc::UnboundedSender<ProviderEvent>) -> Self {
        let (shutdown, _) = tokio::sync::watch::channel(false);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            shutdown,
            discovery: tokio::sync::Mutex::new(None),
        }
    }

    pub async fn start(&self) {
        let sessions = self.sessions.clone();
        let event_tx = self.event_tx.clone();
        let mut shutdown_rx = self.shutdown.subscribe();

        // Discovery channel
        let (discovery_tx, mut discovery_rx) = mpsc::unbounded_channel();
        let mut discovery = SessionDiscovery::new(discovery_tx);
        discovery.start().await;

        // Store discovery to keep its shutdown channel alive
        *self.discovery.lock().await = Some(discovery);

        // Handle discovery events
        let sessions_clone = sessions.clone();
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = discovery_rx.recv().await {
                match event {
                    DiscoveryEvent::Found(discovered) => {
                        handle_session_found(
                            &sessions_clone,
                            &event_tx_clone,
                            discovered,
                        )
                        .await;
                    }
                    DiscoveryEvent::Removed(session_id) => {
                        let mut sessions = sessions_clone.write().await;
                        if let Some(session) = sessions.remove(&session_id) {
                            session.watcher.stop();
                            let _ = event_tx_clone.send(ProviderEvent::SessionRemoved {
                                session_id,
                            });
                        }
                    }
                }
            }
        });

        // Periodic timer check (3s)
        let sessions_timer = self.sessions.clone();
        let event_tx_timer = self.event_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => break,
                    _ = interval.tick() => {
                        check_timers(&sessions_timer, &event_tx_timer).await;
                    }
                }
            }
        });
    }

    pub async fn stop(&self) {
        let _ = self.shutdown.send(true);
        if let Some(discovery) = self.discovery.lock().await.take() {
            discovery.stop();
        }
        let mut sessions = self.sessions.write().await;
        for (_, session) in sessions.drain() {
            session.watcher.stop();
        }
    }

    pub async fn get_sessions(&self) -> Vec<AgentSessionSummary> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.emitted)
            .map(|s| s.summary.clone())
            .collect()
    }

    pub async fn get_session_detail(&self, session_id: &str) -> Option<AgentSessionDetail> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| AgentSessionDetail {
            summary: s.summary.clone(),
            messages: s.messages.clone(),
        })
    }

    pub async fn get_session_messages(&self, session_id: &str) -> Option<Vec<AgentMessage>> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.messages.clone())
    }

    pub async fn search_sessions(
        &self,
        query: &str,
        scopes: &[SearchScope],
    ) -> Vec<SessionSearchResult> {
        let query_lower = query.to_lowercase();
        let sessions = self.sessions.read().await;
        let mut results: Vec<SessionSearchResult> = Vec::new();

        for session in sessions.values().filter(|s| s.emitted) {
            let mut matches: Vec<SearchMatch> = Vec::new();

            for scope in scopes {
                match scope {
                    SearchScope::ProjectName => {
                        if session.summary.project_name.to_lowercase().contains(&query_lower) {
                            matches.push(SearchMatch {
                                content: session.summary.project_name.clone(),
                                scope: SearchScope::ProjectName,
                                message_role: MessageRole::System,
                                message_type: MessageType::Text,
                                timestamp: session.summary.started_at.clone(),
                            });
                        }
                    }
                    SearchScope::CurrentTask => {
                        if session.summary.current_task.to_lowercase().contains(&query_lower) {
                            matches.push(SearchMatch {
                                content: make_snippet(
                                    &session.summary.current_task,
                                    &query_lower,
                                ),
                                scope: SearchScope::CurrentTask,
                                message_role: MessageRole::System,
                                message_type: MessageType::Text,
                                timestamp: session.summary.started_at.clone(),
                            });
                        }
                    }
                    SearchScope::WorkingDirectory => {
                        let wd = &session.summary.working_directory;
                        let pp = &session.summary.project_path;
                        if wd.to_lowercase().contains(&query_lower)
                            || pp.to_lowercase().contains(&query_lower)
                        {
                            matches.push(SearchMatch {
                                content: wd.clone(),
                                scope: SearchScope::WorkingDirectory,
                                message_role: MessageRole::System,
                                message_type: MessageType::Text,
                                timestamp: session.summary.started_at.clone(),
                            });
                        }
                    }
                    SearchScope::Content => {
                        for msg in &session.messages {
                            if msg.content.to_lowercase().contains(&query_lower) {
                                matches.push(SearchMatch {
                                    content: make_snippet(&msg.content, &query_lower),
                                    scope: SearchScope::Content,
                                    message_role: msg.role,
                                    message_type: msg.msg_type,
                                    timestamp: msg.timestamp.clone(),
                                });
                            }
                        }
                    }
                }
            }

            if !matches.is_empty() {
                let match_count = matches.len() as u32;
                matches.truncate(3);
                results.push(SessionSearchResult {
                    session: session.summary.clone(),
                    match_count,
                    matches,
                });
            }
        }

        results.sort_by(|a, b| b.match_count.cmp(&a.match_count));
        results
    }
}

fn make_snippet(text: &str, query_lower: &str) -> String {
    let text_lower = text.to_lowercase();
    let Some(pos) = text_lower.find(query_lower) else {
        // Shouldn't happen, but fall back to truncation
        return truncate_str(text, 100);
    };

    let context = 40;
    let start = if pos > context {
        // Find a safe char boundary
        let mut s = pos - context;
        while s > 0 && !text.is_char_boundary(s) {
            s -= 1;
        }
        s
    } else {
        0
    };

    let end = {
        let mut e = (pos + query_lower.len() + context).min(text.len());
        while e < text.len() && !text.is_char_boundary(e) {
            e += 1;
        }
        e
    };

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(&text[start..end]);
    if end < text.len() {
        snippet.push_str("...");
    }
    snippet
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    let mut end = max_len;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

async fn handle_session_found(
    sessions: &Arc<RwLock<HashMap<String, TrackedSession>>>,
    event_tx: &mpsc::UnboundedSender<ProviderEvent>,
    discovered: DiscoveredSession,
) {
    {
        let sessions_read = sessions.read().await;
        if sessions_read.contains_key(&discovered.session_id) {
            return;
        }
    }

    // Stop older active sessions for the same project.
    // When a new session appears (e.g. user selected "clear session"),
    // the old session's JSONL stops receiving entries, so we mark it Stopped
    // immediately rather than waiting for the 30-minute timeout.
    {
        let mut sessions_write = sessions.write().await;
        for (sid, session) in sessions_write.iter_mut() {
            if session.discovery_project_path == discovered.project_path
                && *sid != discovered.session_id
                && matches!(
                    session.state_ctx.state,
                    AgentStateType::Running
                        | AgentStateType::Idle
                        | AgentStateType::PermissionWaiting
                        | AgentStateType::Error
                )
            {
                let prev = session.state_ctx.state;
                session.state_ctx.state = AgentStateType::Stopped;
                session.summary.state = AgentStateType::Stopped;
                if session.emitted {
                    let _ = event_tx.send(ProviderEvent::StateChanged {
                        session_id: sid.clone(),
                        previous: prev,
                        current: AgentStateType::Stopped,
                    });
                }
            }
        }
    }

    let state_ctx = StateContext::new();
    let summary = AgentSessionSummary {
        session_id: discovered.session_id.clone(),
        provider: "claude-code".to_string(),
        state: AgentStateType::Stopped,
        project_path: discovered.project_path.clone(),
        project_name: discovered.project_name.clone(),
        working_directory: discovered.project_path.clone(),
        current_task: String::new(),
        model: "unknown".to_string(),
        last_activity_at: chrono::Utc::now().to_rfc3339(),
        started_at: chrono::Utc::now().to_rfc3339(),
        cumulative_usage: CumulativeUsage::default(),
        git_status: GitStatus::default(),
    };

    // Create file watcher with entry channel
    let (entries_tx, mut entries_rx) = mpsc::unbounded_channel();
    let watcher = FileWatcher::new(discovered.log_file.clone(), entries_tx);
    watcher.start().await;

    let tracked = TrackedSession {
        summary,
        messages: Vec::new(),
        state_ctx,
        watcher,
        model: "unknown".to_string(),
        emitted: false,
        last_git_diff_check: 0,
        discovery_project_path: discovered.project_path.clone(),
    };

    {
        let mut sessions_write = sessions.write().await;
        sessions_write.insert(discovered.session_id.clone(), tracked);
    }

    // Spawn task to handle entries from file watcher
    let sessions_clone = sessions.clone();
    let event_tx_clone = event_tx.clone();
    let session_id = discovered.session_id.clone();
    tokio::spawn(async move {
        while let Some(entries) = entries_rx.recv().await {
            handle_entries(&sessions_clone, &event_tx_clone, &session_id, entries).await;
        }
    });
}

async fn handle_entries(
    sessions: &Arc<RwLock<HashMap<String, TrackedSession>>>,
    event_tx: &mpsc::UnboundedSender<ProviderEvent>,
    session_id: &str,
    entries: Vec<RawEntry>,
) {
    let mut sessions = sessions.write().await;
    let session = match sessions.get_mut(session_id) {
        Some(s) => s,
        None => return,
    };

    for entry in &entries {
        // Extract metadata from user messages
        if let RawEntry::User(user_msg) = entry {
            if let Some(cwd) = &user_msg.cwd {
                session.summary.working_directory = cwd.clone();
                if let Some(name) = cwd.split('/').last() {
                    if !name.is_empty() {
                        session.summary.project_name = name.to_string();
                    }
                }
                session.summary.project_path = cwd.clone();
            }
            if session.summary.current_task.is_empty() {
                let (_, _, current_task) = extract_session_metadata(user_msg);
                session.summary.current_task = current_task;
                if let Some(ts) = &user_msg.timestamp {
                    session.summary.started_at = ts.clone();
                }
            }
            // Extract git branch
            if let Some(branch) = &user_msg.git_branch {
                if !branch.is_empty() && branch != "HEAD" {
                    session.summary.git_status.branch = branch.clone();
                }
            }
        }

        // Extract git branch and model from assistant messages
        if let RawEntry::Assistant(assistant_msg) = entry {
            if let Some(branch) = &assistant_msg.git_branch {
                if !branch.is_empty() && branch != "HEAD" {
                    session.summary.git_status.branch = branch.clone();
                }
            }
            let model = extract_model(assistant_msg);
            if model != "unknown" {
                let was_unknown = session.model == "unknown";
                session.model = model.to_string();
                session.summary.model = model.to_string();

                if was_unknown && !session.emitted {
                    session.emitted = true;
                    let _ = event_tx.send(ProviderEvent::SessionDiscovered {
                        session: session.summary.clone(),
                    });
                }
            }

            // Update usage
            if let Some((input, output, cache_read, cache_creation)) = extract_usage(assistant_msg) {
                session.summary.cumulative_usage = add_usage(
                    &session.summary.cumulative_usage,
                    &session.model,
                    input,
                    output,
                    cache_read,
                    cache_creation,
                );
                if session.emitted {
                    let _ = event_tx.send(ProviderEvent::UsageUpdated {
                        session_id: session_id.to_string(),
                        usage: session.summary.cumulative_usage.clone(),
                    });
                }
            }
        }

        // Process state machine
        let prev_state = session.state_ctx.state;
        let result = process_entry(&mut session.state_ctx, entry);

        if result.changed {
            session.summary.state = session.state_ctx.state;
            session.summary.last_activity_at =
                chrono::DateTime::from_timestamp_millis(session.state_ctx.last_entry_timestamp)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            if session.emitted {
                let _ = event_tx.send(ProviderEvent::StateChanged {
                    session_id: session_id.to_string(),
                    previous: prev_state,
                    current: session.state_ctx.state,
                });
            }
        }

        // Map to AgentMessages
        let messages = map_entry(entry, session_id);
        for msg in messages {
            session.messages.push(msg.clone());
            // Keep only last 500 messages
            if session.messages.len() > 500 {
                let drain_count = session.messages.len() - 400;
                session.messages.drain(..drain_count);
            }
            if session.emitted {
                let _ = event_tx.send(ProviderEvent::NewMessage {
                    session_id: session_id.to_string(),
                    message: msg,
                });
            }
        }
    }

    session.summary.last_activity_at =
        chrono::DateTime::from_timestamp_millis(session.state_ctx.last_entry_timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    // Run time-based check immediately after processing entries so that
    // stale sessions (e.g. Idle for hours) get the correct state on startup
    // instead of waiting for the next 3s timer tick.
    let prev_state = session.state_ctx.state;
    let result = check_time_based_transitions(&mut session.state_ctx);
    if result.changed {
        session.summary.state = session.state_ctx.state;
        if session.emitted {
            let _ = event_tx.send(ProviderEvent::StateChanged {
                session_id: session_id.to_string(),
                previous: prev_state,
                current: session.state_ctx.state,
            });
        }
    }
}

async fn fetch_git_diff_stats(working_directory: &str) -> Option<(u64, u64)> {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--shortstat"])
        .current_dir(working_directory)
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_shortstat(&String::from_utf8_lossy(&output.stdout))
}

fn parse_shortstat(output: &str) -> Option<(u64, u64)> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Some((0, 0));
    }
    let mut additions: u64 = 0;
    let mut deletions: u64 = 0;
    // " 3 files changed, 42 insertions(+), 10 deletions(-)"
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.contains("insertion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                additions = n;
            }
        } else if part.contains("deletion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                deletions = n;
            }
        }
    }
    Some((additions, deletions))
}

async fn check_timers(
    sessions: &Arc<RwLock<HashMap<String, TrackedSession>>>,
    event_tx: &mpsc::UnboundedSender<ProviderEvent>,
) {
    // Collect git diff targets while holding the lock
    let mut git_diff_targets: Vec<(String, String)> = Vec::new();

    {
        let mut sessions = sessions.write().await;
        let now_ms = chrono::Utc::now().timestamp_millis();

        for (session_id, session) in sessions.iter_mut() {
            if !session.emitted {
                continue;
            }

            // State transitions
            let prev_state = session.state_ctx.state;
            let result = check_time_based_transitions(&mut session.state_ctx);

            if result.changed {
                session.summary.state = session.state_ctx.state;
                let _ = event_tx.send(ProviderEvent::StateChanged {
                    session_id: session_id.clone(),
                    previous: prev_state,
                    current: session.state_ctx.state,
                });
            }

            // Check if git diff is needed
            let state = session.state_ctx.state;
            if (state == AgentStateType::Idle || state == AgentStateType::PermissionWaiting)
                && !session.summary.working_directory.is_empty()
                && (now_ms - session.last_git_diff_check) > 30_000
            {
                session.last_git_diff_check = now_ms;
                git_diff_targets.push((
                    session_id.clone(),
                    session.summary.working_directory.clone(),
                ));
            }
        }
    }
    // Lock released — run git diff concurrently
    if !git_diff_targets.is_empty() {
        let mut handles = Vec::new();
        for (session_id, wd) in git_diff_targets {
            let event_tx = event_tx.clone();
            let sessions = sessions.clone();
            handles.push(tokio::spawn(async move {
                if let Some((additions, deletions)) = fetch_git_diff_stats(&wd).await {
                    let mut sessions = sessions.write().await;
                    if let Some(session) = sessions.get_mut(&session_id) {
                        let changed = session.summary.git_status.additions != additions
                            || session.summary.git_status.deletions != deletions;
                        if changed {
                            session.summary.git_status.additions = additions;
                            session.summary.git_status.deletions = deletions;
                            let _ = event_tx.send(ProviderEvent::GitStatusUpdated {
                                session_id: session_id.clone(),
                                git_status: session.summary.git_status.clone(),
                            });
                        }
                    }
                }
            }));
        }
        for handle in handles {
            let _ = handle.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortstat_full() {
        let output = " 3 files changed, 42 insertions(+), 10 deletions(-)";
        assert_eq!(parse_shortstat(output), Some((42, 10)));
    }

    #[test]
    fn test_parse_shortstat_insertions_only() {
        let output = " 1 file changed, 5 insertions(+)";
        assert_eq!(parse_shortstat(output), Some((5, 0)));
    }

    #[test]
    fn test_parse_shortstat_deletions_only() {
        let output = " 2 files changed, 3 deletions(-)";
        assert_eq!(parse_shortstat(output), Some((0, 3)));
    }

    #[test]
    fn test_parse_shortstat_empty() {
        assert_eq!(parse_shortstat(""), Some((0, 0)));
        assert_eq!(parse_shortstat("  "), Some((0, 0)));
    }
}

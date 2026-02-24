pub mod claude_code;

use crate::types::{AgentMessage, AgentSessionSummary, AgentStateType, CumulativeUsage, GitStatus};

#[derive(Debug, Clone)]
pub enum ProviderEvent {
    SessionDiscovered {
        session: AgentSessionSummary,
    },
    SessionRemoved {
        session_id: String,
    },
    StateChanged {
        session_id: String,
        previous: AgentStateType,
        current: AgentStateType,
    },
    NewMessage {
        session_id: String,
        message: AgentMessage,
    },
    UsageUpdated {
        session_id: String,
        usage: CumulativeUsage,
    },
    GitStatusUpdated {
        session_id: String,
        git_status: GitStatus,
    },
}

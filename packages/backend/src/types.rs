use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Agent State ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentStateType {
    Running,
    Idle,
    PermissionWaiting,
    Error,
    Stopped,
}

impl std::fmt::Display for AgentStateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStateType::Running => write!(f, "running"),
            AgentStateType::Idle => write!(f, "idle"),
            AgentStateType::PermissionWaiting => write!(f, "permission_waiting"),
            AgentStateType::Error => write!(f, "error"),
            AgentStateType::Stopped => write!(f, "stopped"),
        }
    }
}

// ── Usage ──

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CumulativeUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub estimated_cost: f64,
}

// ── Git Status ──

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: String,
    pub additions: u64,
    pub deletions: u64,
}

// ── Session Summary ──

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSummary {
    pub session_id: String,
    pub provider: String,
    pub state: AgentStateType,
    pub project_path: String,
    pub project_name: String,
    pub working_directory: String,
    pub current_task: String,
    pub model: String,
    pub last_activity_at: String,
    pub started_at: String,
    pub cumulative_usage: CumulativeUsage,
    pub git_status: GitStatus,
}

// ── Messages ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    ToolUse,
    ToolResult,
    Thinking,
    StateChange,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub session_id: String,
    pub timestamp: String,
    pub role: MessageRole,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

// ── Session Detail ──

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionDetail {
    #[serde(flatten)]
    pub summary: AgentSessionSummary,
    pub messages: Vec<AgentMessage>,
}

// ── Search ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchScope {
    ProjectName,
    CurrentTask,
    WorkingDirectory,
    Content,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchMatch {
    pub content: String,
    pub scope: SearchScope,
    pub message_role: MessageRole,
    pub message_type: MessageType,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionSearchResult {
    pub session: AgentSessionSummary,
    pub match_count: u32,
    pub matches: Vec<SearchMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query: String,
    pub total_sessions: u32,
    pub results: Vec<SessionSearchResult>,
}

// ── WebSocket Protocol ──

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "sessions:init")]
    SessionsInit {
        sessions: Vec<AgentSessionSummary>,
    },

    #[serde(rename = "session:discovered")]
    SessionDiscovered {
        session: AgentSessionSummary,
    },

    #[serde(rename = "session:removed")]
    #[serde(rename_all = "camelCase")]
    SessionRemoved {
        session_id: String,
    },

    #[serde(rename = "session:state_changed")]
    #[serde(rename_all = "camelCase")]
    StateChanged {
        session_id: String,
        previous: AgentStateType,
        current: AgentStateType,
        session: AgentSessionSummary,
    },

    #[serde(rename = "session:new_message")]
    #[serde(rename_all = "camelCase")]
    NewMessage {
        session_id: String,
        message: AgentMessage,
    },

    #[serde(rename = "session:messages_init")]
    #[serde(rename_all = "camelCase")]
    MessagesInit {
        session_id: String,
        messages: Vec<AgentMessage>,
    },

    #[serde(rename = "session:usage_updated")]
    #[serde(rename_all = "camelCase")]
    UsageUpdated {
        session_id: String,
        usage: CumulativeUsage,
    },

    #[serde(rename = "session:git_status_updated")]
    #[serde(rename_all = "camelCase")]
    GitStatusUpdated {
        session_id: String,
        git_status: GitStatus,
    },
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "subscribe:session")]
    #[serde(rename_all = "camelCase")]
    Subscribe { session_id: String },

    #[serde(rename = "unsubscribe:session")]
    #[serde(rename_all = "camelCase")]
    Unsubscribe { session_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_serialization() {
        assert_eq!(
            serde_json::to_string(&AgentStateType::Running).unwrap(),
            r#""running""#
        );
        assert_eq!(
            serde_json::to_string(&AgentStateType::PermissionWaiting).unwrap(),
            r#""permission_waiting""#
        );
    }

    #[test]
    fn test_cumulative_usage_camel_case() {
        let usage = CumulativeUsage {
            input_tokens: 100,
            output_tokens: 200,
            cache_read_tokens: 50,
            cache_creation_tokens: 25,
            estimated_cost: 0.01,
        };
        let json = serde_json::to_value(&usage).unwrap();
        assert!(json.get("inputTokens").is_some());
        assert!(json.get("outputTokens").is_some());
        assert!(json.get("cacheReadTokens").is_some());
        assert!(json.get("cacheCreationTokens").is_some());
        assert!(json.get("estimatedCost").is_some());
    }

    #[test]
    fn test_session_summary_camel_case() {
        let summary = AgentSessionSummary {
            session_id: "abc".into(),
            provider: "claude-code".into(),
            state: AgentStateType::Running,
            project_path: "/tmp".into(),
            project_name: "test".into(),
            working_directory: "/tmp".into(),
            current_task: "hello".into(),
            model: "claude-sonnet-4-20250514".into(),
            last_activity_at: "2025-01-01T00:00:00Z".into(),
            started_at: "2025-01-01T00:00:00Z".into(),
            cumulative_usage: CumulativeUsage::default(),
            git_status: GitStatus::default(),
        };
        let json = serde_json::to_value(&summary).unwrap();
        assert!(json.get("sessionId").is_some());
        assert!(json.get("projectPath").is_some());
        assert!(json.get("projectName").is_some());
        assert!(json.get("workingDirectory").is_some());
        assert!(json.get("currentTask").is_some());
        assert!(json.get("lastActivityAt").is_some());
        assert!(json.get("startedAt").is_some());
        assert!(json.get("cumulativeUsage").is_some());
    }

    #[test]
    fn test_server_event_sessions_init() {
        let event = ServerEvent::SessionsInit {
            sessions: vec![],
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "sessions:init");
        assert!(json["sessions"].is_array());
    }

    #[test]
    fn test_server_event_state_changed() {
        let event = ServerEvent::StateChanged {
            session_id: "s1".into(),
            previous: AgentStateType::Running,
            current: AgentStateType::Idle,
            session: AgentSessionSummary {
                session_id: "s1".into(),
                provider: "claude-code".into(),
                state: AgentStateType::Idle,
                project_path: "/tmp".into(),
                project_name: "test".into(),
                working_directory: "/tmp".into(),
                current_task: "".into(),
                model: "claude-sonnet-4-20250514".into(),
                last_activity_at: "2025-01-01T00:00:00Z".into(),
                started_at: "2025-01-01T00:00:00Z".into(),
                cumulative_usage: CumulativeUsage::default(),
                git_status: GitStatus::default(),
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session:state_changed");
        assert_eq!(json["sessionId"], "s1");
        assert_eq!(json["previous"], "running");
        assert_eq!(json["current"], "idle");
        assert!(json["session"].is_object());
    }

    #[test]
    fn test_server_event_new_message() {
        let event = ServerEvent::NewMessage {
            session_id: "s1".into(),
            message: AgentMessage {
                id: "msg_1".into(),
                session_id: "s1".into(),
                timestamp: "2025-01-01T00:00:00Z".into(),
                role: MessageRole::User,
                msg_type: MessageType::Text,
                content: "hello".into(),
                metadata: None,
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session:new_message");
        assert_eq!(json["sessionId"], "s1");
        assert_eq!(json["message"]["type"], "text");
        assert_eq!(json["message"]["role"], "user");
    }

    #[test]
    fn test_client_event_subscribe() {
        let json = r#"{"type":"subscribe:session","sessionId":"abc123"}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::Subscribe { session_id } => {
                assert_eq!(session_id, "abc123");
            }
            _ => panic!("Expected Subscribe"),
        }
    }

    #[test]
    fn test_client_event_unsubscribe() {
        let json = r#"{"type":"unsubscribe:session","sessionId":"abc123"}"#;
        let event: ClientEvent = serde_json::from_str(json).unwrap();
        match event {
            ClientEvent::Unsubscribe { session_id } => {
                assert_eq!(session_id, "abc123");
            }
            _ => panic!("Expected Unsubscribe"),
        }
    }

    #[test]
    fn test_agent_message_type_field_name() {
        let msg = AgentMessage {
            id: "1".into(),
            session_id: "s1".into(),
            timestamp: "2025-01-01T00:00:00Z".into(),
            role: MessageRole::Assistant,
            msg_type: MessageType::ToolUse,
            content: "Read".into(),
            metadata: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        // Must use "type" not "msgType"
        assert_eq!(json["type"], "tool_use");
        assert!(json.get("msgType").is_none());
    }

    #[test]
    fn test_session_removed_event() {
        let event = ServerEvent::SessionRemoved {
            session_id: "s1".into(),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session:removed");
        assert_eq!(json["sessionId"], "s1");
    }

    #[test]
    fn test_usage_updated_event() {
        let event = ServerEvent::UsageUpdated {
            session_id: "s1".into(),
            usage: CumulativeUsage {
                input_tokens: 100,
                output_tokens: 200,
                cache_read_tokens: 50,
                cache_creation_tokens: 25,
                estimated_cost: 0.01,
            },
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "session:usage_updated");
        assert_eq!(json["sessionId"], "s1");
        assert_eq!(json["usage"]["inputTokens"], 100);
    }
}

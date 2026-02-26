use crate::types::AgentStateType;
use super::jsonl_parser::{RawContentBlock, RawEntry, RawUserMessage};
use chrono::{DateTime, Utc};

const PERMISSION_WAIT_TIMEOUT_MS: i64 = 30_000;
const IDLE_TIMEOUT_MS: i64 = 10_000;
const STOPPED_TIMEOUT_MS: i64 = 1_800_000; // 30 minutes
const IDLE_STOPPED_TIMEOUT_MS: i64 = 1_800_000; // 30 minutes

#[derive(Debug, Clone)]
pub struct StateContext {
    pub state: AgentStateType,
    pub last_activity_at: i64, // millis since epoch
    pub last_assistant_tool_use: bool,
    pub last_assistant_text_only: bool,
    pub last_entry_timestamp: i64,
}

impl StateContext {
    pub fn new() -> Self {
        Self {
            state: AgentStateType::Stopped,
            last_activity_at: 0,
            last_assistant_tool_use: false,
            last_assistant_text_only: false,
            last_entry_timestamp: 0,
        }
    }
}

pub struct TransitionResult {
    pub new_state: AgentStateType,
    pub changed: bool,
}

fn has_tool_use_block(blocks: &[RawContentBlock]) -> bool {
    blocks.iter().any(|b| matches!(b, RawContentBlock::ToolUse { .. }))
}

fn has_error_pattern(entry: &RawEntry) -> bool {
    if let RawEntry::User(user_msg) = entry {
        if let Some(arr) = user_msg.message.content.as_array() {
            for block in arr {
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                    if block.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_exit_command_entry(msg: &RawUserMessage) -> bool {
    if let Some(content_str) = msg.message.content.as_str() {
        return content_str.contains("<command-name>/exit</command-name>");
    }
    false
}

fn is_local_command_entry(msg: &RawUserMessage) -> bool {
    if let Some(content_str) = msg.message.content.as_str() {
        return content_str.contains("<local-command-stdout>")
            || content_str.contains("<local-command-caveat>")
            || content_str.contains("<command-name>");
    }
    false
}

fn get_entry_timestamp(entry: &RawEntry) -> Option<i64> {
    let ts_str = match entry {
        RawEntry::User(m) => m.timestamp.as_deref(),
        RawEntry::Assistant(m) => m.timestamp.as_deref(),
        RawEntry::System(m) => m.timestamp.as_deref(),
        RawEntry::Progress(m) => m.timestamp.as_deref(),
        RawEntry::Other => None,
    };

    ts_str.and_then(|s| {
        s.parse::<DateTime<Utc>>()
            .ok()
            .map(|dt| dt.timestamp_millis())
    })
}

pub fn process_entry(ctx: &mut StateContext, entry: &RawEntry) -> TransitionResult {
    let prev_state = ctx.state;
    let now = Utc::now().timestamp_millis();

    let entry_ts = get_entry_timestamp(entry);
    if let Some(ts) = entry_ts {
        ctx.last_activity_at = ts;
        ctx.last_entry_timestamp = ts;
    }

    // Handle system:turn_duration → Idle
    if let RawEntry::System(sys) = entry {
        if sys.subtype.as_deref() == Some("turn_duration") {
            ctx.state = AgentStateType::Idle;
            ctx.last_assistant_tool_use = false;
            ctx.last_assistant_text_only = false;
            return TransitionResult {
                new_state: ctx.state,
                changed: prev_state != ctx.state,
            };
        }
    }

    // Handle user message → Running (unless local command)
    if let RawEntry::User(user_msg) = entry {
        if is_exit_command_entry(user_msg) {
            ctx.state = AgentStateType::Stopped;
            ctx.last_assistant_tool_use = false;
            ctx.last_assistant_text_only = false;
            return TransitionResult {
                new_state: ctx.state,
                changed: prev_state != ctx.state,
            };
        }
        if is_local_command_entry(user_msg) {
            return TransitionResult {
                new_state: ctx.state,
                changed: false,
            };
        }
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_tool_use = false;
        ctx.last_assistant_text_only = false;
        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    // Handle assistant message
    if let RawEntry::Assistant(assistant_msg) = entry {
        ctx.state = AgentStateType::Running;

        if has_tool_use_block(&assistant_msg.message.content) {
            ctx.last_assistant_tool_use = true;
            ctx.last_assistant_text_only = false;
        } else {
            ctx.last_assistant_tool_use = false;
            ctx.last_assistant_text_only = true;
        }

        if has_error_pattern(entry) {
            ctx.state = AgentStateType::Error;
        }

        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    // Progress entries → Running
    // Progress entries indicate a tool is actively executing (approved),
    // so clear tool_use flag to prevent false PermissionWaiting detection.
    if matches!(entry, RawEntry::Progress(_)) {
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_tool_use = false;
        ctx.last_assistant_text_only = false;
        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    TransitionResult {
        new_state: ctx.state,
        changed: false,
    }
}

pub fn check_time_based_transitions(ctx: &mut StateContext) -> TransitionResult {
    let prev_state = ctx.state;
    let now = Utc::now().timestamp_millis();
    let elapsed = now - ctx.last_activity_at;

    // If last entry was text-only assistant and silence > 10s → Idle
    if ctx.state == AgentStateType::Running
        && ctx.last_assistant_text_only
        && elapsed >= IDLE_TIMEOUT_MS
    {
        ctx.state = AgentStateType::Idle;
        ctx.last_assistant_text_only = false;
        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    // If last entry was tool_use and silence > 10s → PermissionWaiting
    if ctx.state == AgentStateType::Running
        && ctx.last_assistant_tool_use
        && elapsed >= PERMISSION_WAIT_TIMEOUT_MS
    {
        ctx.state = AgentStateType::PermissionWaiting;
        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    // If no activity for 60s AND was running → Stopped
    if elapsed >= STOPPED_TIMEOUT_MS && ctx.state == AgentStateType::Running {
        ctx.state = AgentStateType::Stopped;
        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    // If no activity for 5min AND was idle → Stopped
    if elapsed >= IDLE_STOPPED_TIMEOUT_MS && ctx.state == AgentStateType::Idle {
        ctx.state = AgentStateType::Stopped;
        return TransitionResult {
            new_state: ctx.state,
            changed: prev_state != ctx.state,
        };
    }

    TransitionResult {
        new_state: ctx.state,
        changed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::claude_code::jsonl_parser::*;
    use serde_json::json;

    fn make_user_entry(content: &str) -> RawEntry {
        RawEntry::User(RawUserMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: Some("1.0".into()),
            cwd: Some("/tmp".into()),
            slug: None,
            message: RawUserMessageBody {
                role: "user".into(),
                content: json!(content),
            },
            uuid: Some("u1".into()),
            timestamp: Some(Utc::now().to_rfc3339()),
            git_branch: None,
        })
    }

    fn make_assistant_entry(blocks: Vec<RawContentBlock>) -> RawEntry {
        RawEntry::Assistant(RawAssistantMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: None,
            slug: None,
            message: RawAssistantMessageBody {
                model: Some("claude-sonnet-4-20250514".into()),
                id: None,
                content: blocks,
                stop_reason: None,
                usage: None,
            },
            uuid: Some("a1".into()),
            timestamp: Some(Utc::now().to_rfc3339()),
            git_branch: None,
        })
    }

    fn make_system_turn_duration() -> RawEntry {
        RawEntry::System(RawSystemEntry {
            subtype: Some("turn_duration".into()),
            session_id: Some("s1".into()),
            timestamp: Some(Utc::now().to_rfc3339()),
            duration_ms: Some(1500),
        })
    }

    #[test]
    fn test_initial_state() {
        let ctx = StateContext::new();
        assert_eq!(ctx.state, AgentStateType::Stopped);
    }

    #[test]
    fn test_user_message_transitions_to_running() {
        let mut ctx = StateContext::new();
        let entry = make_user_entry("hello");
        let result = process_entry(&mut ctx, &entry);
        assert_eq!(result.new_state, AgentStateType::Running);
        assert!(result.changed);
    }

    #[test]
    fn test_assistant_message_transitions_to_running() {
        let mut ctx = StateContext::new();
        let entry = make_assistant_entry(vec![RawContentBlock::Text {
            text: "hello".into(),
        }]);
        let result = process_entry(&mut ctx, &entry);
        assert_eq!(result.new_state, AgentStateType::Running);
        assert!(result.changed);
    }

    #[test]
    fn test_turn_duration_transitions_to_idle() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        let entry = make_system_turn_duration();
        let result = process_entry(&mut ctx, &entry);
        assert_eq!(result.new_state, AgentStateType::Idle);
        assert!(result.changed);
    }

    #[test]
    fn test_tool_use_sets_flag() {
        let mut ctx = StateContext::new();
        let entry = make_assistant_entry(vec![RawContentBlock::ToolUse {
            id: "t1".into(),
            name: "Read".into(),
            input: json!({}),
            caller: None,
        }]);
        process_entry(&mut ctx, &entry);
        assert!(ctx.last_assistant_tool_use);
    }

    #[test]
    fn test_local_command_ignored() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Idle;
        let entry = make_user_entry("some <local-command-stdout> output");
        let result = process_entry(&mut ctx, &entry);
        assert_eq!(result.new_state, AgentStateType::Idle);
        assert!(!result.changed);
    }

    #[test]
    fn test_exit_command_transitions_to_stopped() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        let entry = make_user_entry("<command-name>/exit</command-name>\n            <command-message>exit</command-message>\n            <command-args></command-args>");
        let result = process_entry(&mut ctx, &entry);
        assert_eq!(result.new_state, AgentStateType::Stopped);
        assert!(result.changed);
    }

    #[test]
    fn test_other_command_name_entry_ignored() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Idle;
        let entry = make_user_entry("<command-name>/help</command-name>\n            <command-message>help</command-message>");
        let result = process_entry(&mut ctx, &entry);
        assert_eq!(result.new_state, AgentStateType::Idle);
        assert!(!result.changed);
    }

    #[test]
    fn test_permission_wait_timeout() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_tool_use = true;
        // Set last_activity_at to 35 seconds ago (past 30s threshold)
        ctx.last_activity_at = Utc::now().timestamp_millis() - 35_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::PermissionWaiting);
        assert!(result.changed);
    }

    #[test]
    fn test_no_permission_wait_before_timeout() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_tool_use = true;
        // Set last_activity_at to 20 seconds ago (within 30s threshold)
        ctx.last_activity_at = Utc::now().timestamp_millis() - 20_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::Running);
        assert!(!result.changed);
    }

    #[test]
    fn test_stopped_timeout() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_tool_use = false;
        // Set last_activity_at to 31 minutes ago
        ctx.last_activity_at = Utc::now().timestamp_millis() - 1_860_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::Stopped);
        assert!(result.changed);
    }

    #[test]
    fn test_idle_does_not_transition_to_stopped_before_30min() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Idle;
        // 20 minutes ago — still within 30min threshold
        ctx.last_activity_at = Utc::now().timestamp_millis() - 1_200_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::Idle);
        assert!(!result.changed);
    }

    #[test]
    fn test_idle_transitions_to_stopped_after_30min() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Idle;
        // 31 minutes ago
        ctx.last_activity_at = Utc::now().timestamp_millis() - 1_860_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::Stopped);
        assert!(result.changed);
    }

    #[test]
    fn test_text_only_assistant_sets_flag() {
        let mut ctx = StateContext::new();
        let entry = make_assistant_entry(vec![RawContentBlock::Text {
            text: "Here is the answer.".into(),
        }]);
        process_entry(&mut ctx, &entry);
        assert!(ctx.last_assistant_text_only);
        assert!(!ctx.last_assistant_tool_use);
    }

    #[test]
    fn test_tool_use_clears_text_only_flag() {
        let mut ctx = StateContext::new();
        // First set text_only via a text-only message
        ctx.last_assistant_text_only = true;
        let entry = make_assistant_entry(vec![RawContentBlock::ToolUse {
            id: "t1".into(),
            name: "Read".into(),
            input: json!({}),
            caller: None,
        }]);
        process_entry(&mut ctx, &entry);
        assert!(!ctx.last_assistant_text_only);
        assert!(ctx.last_assistant_tool_use);
    }

    #[test]
    fn test_user_message_clears_text_only_flag() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_text_only = true;
        let entry = make_user_entry("next question");
        process_entry(&mut ctx, &entry);
        assert!(!ctx.last_assistant_text_only);
    }

    #[test]
    fn test_progress_clears_text_only_flag() {
        let mut ctx = StateContext::new();
        ctx.last_assistant_text_only = true;
        let entry = RawEntry::Progress(RawProgressEntry {
            parent_uuid: None,
            data: None,
            uuid: None,
            timestamp: Some(Utc::now().to_rfc3339()),
        });
        process_entry(&mut ctx, &entry);
        assert!(!ctx.last_assistant_text_only);
    }

    #[test]
    fn test_progress_clears_tool_use_flag() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_tool_use = true;
        let entry = RawEntry::Progress(RawProgressEntry {
            parent_uuid: None,
            data: None,
            uuid: None,
            timestamp: Some(Utc::now().to_rfc3339()),
        });
        process_entry(&mut ctx, &entry);
        assert!(!ctx.last_assistant_tool_use);
        assert_eq!(ctx.state, AgentStateType::Running);
    }

    #[test]
    fn test_progress_prevents_permission_wait_false_positive() {
        let mut ctx = StateContext::new();
        // Simulate: assistant sent tool_use, then progress entry arrived
        let tool_entry = make_assistant_entry(vec![RawContentBlock::ToolUse {
            id: "t1".into(),
            name: "Bash".into(),
            input: json!({}),
            caller: None,
        }]);
        process_entry(&mut ctx, &tool_entry);
        assert!(ctx.last_assistant_tool_use);

        // Progress entry arrives (tool is executing)
        let progress_entry = RawEntry::Progress(RawProgressEntry {
            parent_uuid: None,
            data: None,
            uuid: None,
            timestamp: Some(Utc::now().to_rfc3339()),
        });
        process_entry(&mut ctx, &progress_entry);
        assert!(!ctx.last_assistant_tool_use);

        // Even after 35s of silence, should NOT transition to PermissionWaiting
        ctx.last_activity_at = Utc::now().timestamp_millis() - 35_000;
        let result = check_time_based_transitions(&mut ctx);
        assert_ne!(result.new_state, AgentStateType::PermissionWaiting);
    }

    #[test]
    fn test_text_only_idle_timeout() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_text_only = true;
        // 15 seconds ago — past the 10s threshold
        ctx.last_activity_at = Utc::now().timestamp_millis() - 15_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::Idle);
        assert!(result.changed);
        assert!(!ctx.last_assistant_text_only);
    }

    #[test]
    fn test_text_only_no_idle_before_timeout() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_text_only = true;
        // 5 seconds ago — before the 10s threshold
        ctx.last_activity_at = Utc::now().timestamp_millis() - 5_000;

        let result = check_time_based_transitions(&mut ctx);
        assert_eq!(result.new_state, AgentStateType::Running);
        assert!(!result.changed);
    }

    #[test]
    fn test_turn_duration_clears_text_only_flag() {
        let mut ctx = StateContext::new();
        ctx.state = AgentStateType::Running;
        ctx.last_assistant_text_only = true;
        let entry = make_system_turn_duration();
        process_entry(&mut ctx, &entry);
        assert_eq!(ctx.state, AgentStateType::Idle);
        assert!(!ctx.last_assistant_text_only);
    }
}

use crate::types::{AgentMessage, MessageRole, MessageType};
use super::jsonl_parser::{RawAssistantMessage, RawContentBlock, RawEntry, RawUserMessage};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

static MESSAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_id() -> String {
    let id = MESSAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("msg_{}", id + 1)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    // Find a valid char boundary at or before max_len
    let mut end = max_len;
    while !s.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

const SYSTEM_XML_TAGS: &[&str] = &[
    "local-command-caveat",
    "local-command-stdout",
    "command-name",
    "command-message",
    "command-args",
    "system-reminder",
];

fn strip_system_xml_tags(text: &str) -> String {
    let mut result = text.to_string();
    for tag in SYSTEM_XML_TAGS {
        let open = format!("<{}", tag);
        let close = format!("</{}>", tag);
        loop {
            let Some(start) = result.find(&open) else { break };
            if let Some(end_offset) = result[start..].find(&close) {
                let end = start + end_offset + close.len();
                result.replace_range(start..end, "");
            } else {
                // No closing tag found — this is likely a literal mention
                // in user text, not a real system tag. Skip it.
                break;
            }
        }
    }
    result.trim().to_string()
}

fn map_user_message(entry: &RawUserMessage) -> Vec<AgentMessage> {
    let mut messages = Vec::new();
    let session_id = entry.session_id.clone().unwrap_or_default();
    let timestamp = entry.timestamp.clone().unwrap_or_default();
    let uuid = entry.uuid.clone();

    let content = &entry.message.content;

    // String content
    if let Some(text) = content.as_str() {
        messages.push(AgentMessage {
            id: uuid.clone().unwrap_or_else(next_id),
            session_id: session_id.clone(),
            timestamp: timestamp.clone(),
            role: MessageRole::User,
            msg_type: MessageType::Text,
            content: truncate(text, 500),
            metadata: None,
        });
    }
    // Array content (tool results)
    else if let Some(arr) = content.as_array() {
        for block in arr {
            if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                let result_content = block.get("content").map(|c| {
                    if let Some(s) = c.as_str() {
                        s.to_string()
                    } else {
                        serde_json::to_string(c).unwrap_or_default()
                    }
                }).unwrap_or_default();

                let is_error = block.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                let tool_use_id = block.get("tool_use_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

                let mut metadata = std::collections::HashMap::new();
                metadata.insert("toolUseId".to_string(), json!(tool_use_id));
                metadata.insert("isError".to_string(), json!(is_error));

                messages.push(AgentMessage {
                    id: uuid.clone().unwrap_or_else(next_id),
                    session_id: session_id.clone(),
                    timestamp: timestamp.clone(),
                    role: MessageRole::User,
                    msg_type: MessageType::ToolResult,
                    content: truncate(&result_content, 300),
                    metadata: Some(metadata),
                });
            }
        }
    }

    messages
}

fn map_assistant_message(entry: &RawAssistantMessage) -> Vec<AgentMessage> {
    let mut messages = Vec::new();
    let session_id = entry.session_id.clone().unwrap_or_default();
    let timestamp = entry.timestamp.clone().unwrap_or_default();
    let uuid = entry.uuid.clone();

    for block in &entry.message.content {
        match block {
            RawContentBlock::Text { text } => {
                messages.push(AgentMessage {
                    id: uuid.clone().unwrap_or_else(next_id),
                    session_id: session_id.clone(),
                    timestamp: timestamp.clone(),
                    role: MessageRole::Assistant,
                    msg_type: MessageType::Text,
                    content: text.clone(),
                    metadata: None,
                });
            }
            RawContentBlock::ToolUse { id, name, input, .. } => {
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("toolName".to_string(), json!(name));
                metadata.insert("toolId".to_string(), json!(id));
                metadata.insert("input".to_string(), input.clone());

                messages.push(AgentMessage {
                    id: uuid.clone().unwrap_or_else(next_id),
                    session_id: session_id.clone(),
                    timestamp: timestamp.clone(),
                    role: MessageRole::Assistant,
                    msg_type: MessageType::ToolUse,
                    content: name.clone(),
                    metadata: Some(metadata),
                });
            }
            // Skip thinking blocks
            _ => {}
        }
    }

    messages
}

pub fn map_entry(entry: &RawEntry, session_id: &str) -> Vec<AgentMessage> {
    match entry {
        RawEntry::User(user_msg) => map_user_message(user_msg),
        RawEntry::Assistant(assistant_msg) => map_assistant_message(assistant_msg),
        RawEntry::System(sys) => {
            if sys.subtype.as_deref() == Some("turn_duration") {
                let duration_ms = sys.duration_ms.unwrap_or(0);
                let ts = sys.timestamp.clone().unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                let mut metadata = std::collections::HashMap::new();
                metadata.insert("durationMs".to_string(), json!(duration_ms));
                vec![AgentMessage {
                    id: next_id(),
                    session_id: session_id.to_string(),
                    timestamp: ts,
                    role: MessageRole::System,
                    msg_type: MessageType::StateChange,
                    content: format!("Turn completed ({}ms)", duration_ms),
                    metadata: Some(metadata),
                }]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

pub fn extract_session_metadata(entry: &RawUserMessage) -> (String, String, String) {
    let session_id = entry.session_id.clone().unwrap_or_default();
    let cwd = entry.cwd.clone().unwrap_or_default();
    let current_task = if let Some(text) = entry.message.content.as_str() {
        let cleaned = strip_system_xml_tags(text);
        if cleaned.is_empty() {
            String::new()
        } else {
            truncate(&cleaned, 200)
        }
    } else {
        String::new()
    };
    (session_id, cwd, current_task)
}

pub fn extract_model(entry: &RawAssistantMessage) -> &str {
    entry.message.model.as_deref().unwrap_or("unknown")
}

pub fn extract_usage(entry: &RawAssistantMessage) -> Option<(u64, u64, u64, u64)> {
    entry.message.usage.as_ref().map(|u| {
        (
            u.input_tokens,
            u.output_tokens,
            u.cache_read_input_tokens.unwrap_or(0),
            u.cache_creation_input_tokens.unwrap_or(0),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::claude_code::jsonl_parser::*;
    use serde_json::json;

    #[test]
    fn test_map_user_text_message() {
        let entry = RawEntry::User(RawUserMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: None,
            slug: None,
            message: RawUserMessageBody {
                role: "user".into(),
                content: json!("hello world"),
            },
            uuid: Some("u1".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            git_branch: None,
        });
        let msgs = map_entry(&entry, "s1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, MessageRole::User);
        assert_eq!(msgs[0].msg_type, MessageType::Text);
        assert_eq!(msgs[0].content, "hello world");
    }

    #[test]
    fn test_map_user_tool_result() {
        let content = json!([
            {"type": "tool_result", "tool_use_id": "t1", "content": "result data", "is_error": false}
        ]);
        let entry = RawEntry::User(RawUserMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: None,
            slug: None,
            message: RawUserMessageBody {
                role: "user".into(),
                content,
            },
            uuid: Some("u1".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            git_branch: None,
        });
        let msgs = map_entry(&entry, "s1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type, MessageType::ToolResult);
    }

    #[test]
    fn test_map_assistant_text() {
        let entry = RawEntry::Assistant(RawAssistantMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: None,
            slug: None,
            message: RawAssistantMessageBody {
                model: Some("claude-sonnet-4-20250514".into()),
                id: None,
                content: vec![RawContentBlock::Text {
                    text: "hi there".into(),
                }],
                stop_reason: None,
                usage: None,
            },
            uuid: Some("a1".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            git_branch: None,
        });
        let msgs = map_entry(&entry, "s1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, MessageRole::Assistant);
        assert_eq!(msgs[0].content, "hi there");
    }

    #[test]
    fn test_map_assistant_tool_use() {
        let entry = RawEntry::Assistant(RawAssistantMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: None,
            slug: None,
            message: RawAssistantMessageBody {
                model: Some("claude-sonnet-4-20250514".into()),
                id: None,
                content: vec![RawContentBlock::ToolUse {
                    id: "t1".into(),
                    name: "Read".into(),
                    input: json!({"path": "/tmp/file.txt"}),
                    caller: None,
                }],
                stop_reason: None,
                usage: None,
            },
            uuid: Some("a1".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            git_branch: None,
        });
        let msgs = map_entry(&entry, "s1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type, MessageType::ToolUse);
        assert_eq!(msgs[0].content, "Read");
        let meta = msgs[0].metadata.as_ref().unwrap();
        assert_eq!(meta["toolName"], "Read");
    }

    #[test]
    fn test_map_system_turn_duration() {
        let entry = RawEntry::System(RawSystemEntry {
            subtype: Some("turn_duration".into()),
            session_id: Some("s1".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            duration_ms: Some(1500),
        });
        let msgs = map_entry(&entry, "s1");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].msg_type, MessageType::StateChange);
        assert!(msgs[0].content.contains("1500ms"));
    }

    #[test]
    fn test_extract_model() {
        let msg = RawAssistantMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: None,
            version: None,
            cwd: None,
            slug: None,
            message: RawAssistantMessageBody {
                model: Some("claude-opus-4-20250514".into()),
                id: None,
                content: vec![],
                stop_reason: None,
                usage: None,
            },
            uuid: None,
            timestamp: None,
            git_branch: None,
        };
        assert_eq!(extract_model(&msg), "claude-opus-4-20250514");
    }

    #[test]
    fn test_truncation() {
        let long_text = "a".repeat(600);
        let entry = RawEntry::User(RawUserMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: None,
            slug: None,
            message: RawUserMessageBody {
                role: "user".into(),
                content: json!(long_text),
            },
            uuid: Some("u1".into()),
            timestamp: Some("2025-01-01T00:00:00Z".into()),
            git_branch: None,
        });
        let msgs = map_entry(&entry, "s1");
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].content.len() <= 503); // 500 + "..."
    }

    #[test]
    fn test_strip_single_tag() {
        let input = "<local-command-caveat>some caveat</local-command-caveat>Hello world";
        assert_eq!(strip_system_xml_tags(input), "Hello world");
    }

    #[test]
    fn test_strip_multiple_tags() {
        let input = "<command-name>clear</command-name><command-message>msg</command-message>Actual prompt";
        assert_eq!(strip_system_xml_tags(input), "Actual prompt");
    }

    #[test]
    fn test_strip_tags_only() {
        let input = "<local-command-caveat>caveat text here</local-command-caveat>";
        assert_eq!(strip_system_xml_tags(input), "");
    }

    #[test]
    fn test_strip_no_tags() {
        let input = "Plain text with no tags";
        assert_eq!(strip_system_xml_tags(input), "Plain text with no tags");
    }

    #[test]
    fn test_strip_system_reminder() {
        let input = "<system-reminder>reminder content</system-reminder>User prompt";
        assert_eq!(strip_system_xml_tags(input), "User prompt");
    }

    #[test]
    fn test_strip_unclosed_tag_preserves_text() {
        // Unclosed tags are likely literal mentions in user text, not real system tags
        let input = "Hello<local-command-caveat>unclosed tag without end";
        assert_eq!(
            strip_system_xml_tags(input),
            "Hello<local-command-caveat>unclosed tag without end"
        );
    }

    #[test]
    fn test_strip_tag_mentioned_in_user_text() {
        // User mentions a tag name as literal text (e.g. describing an issue)
        let input = r#"Current Task shows "<local-command-caveat>Caveat..." instead of the prompt"#;
        assert_eq!(
            strip_system_xml_tags(input),
            r#"Current Task shows "<local-command-caveat>Caveat..." instead of the prompt"#
        );
    }

    #[test]
    fn test_extract_metadata_with_tags() {
        let entry = RawUserMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: Some("/home/user/project".into()),
            slug: None,
            message: RawUserMessageBody {
                role: "user".into(),
                content: json!("<local-command-caveat>caveat</local-command-caveat>Fix the bug"),
            },
            uuid: None,
            timestamp: None,
            git_branch: None,
        };
        let (sid, cwd, task) = extract_session_metadata(&entry);
        assert_eq!(sid, "s1");
        assert_eq!(cwd, "/home/user/project");
        assert_eq!(task, "Fix the bug");
    }

    #[test]
    fn test_extract_metadata_tags_only() {
        let entry = RawUserMessage {
            parent_uuid: None,
            is_sidechain: None,
            session_id: Some("s1".into()),
            version: None,
            cwd: Some("/home/user/project".into()),
            slug: None,
            message: RawUserMessageBody {
                role: "user".into(),
                content: json!("<local-command-caveat>only caveat text</local-command-caveat>"),
            },
            uuid: None,
            timestamp: None,
            git_branch: None,
        };
        let (_, _, task) = extract_session_metadata(&entry);
        assert_eq!(task, "");
    }
}

use serde::Deserialize;
use serde_json::Value;

// ── Raw content blocks ──

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum RawContentBlock {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "thinking")]
    Thinking {
        thinking: String,
        #[serde(default)]
        signature: Option<String>,
    },

    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
        #[serde(default)]
        caller: Option<Value>,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: Value,
        #[serde(default)]
        is_error: Option<bool>,
    },
}

// ── Raw usage ──

#[derive(Debug, Clone, Deserialize)]
pub struct RawUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
}

// ── Raw message types ──

#[derive(Debug, Clone, Deserialize)]
pub struct RawUserMessageBody {
    pub role: String,
    pub content: Value, // String or Array<RawContentBlock>
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawAssistantMessageBody {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub content: Vec<RawContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub usage: Option<RawUsage>,
}

// ── Raw entries ──

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawUserMessage {
    #[serde(default)]
    pub parent_uuid: Option<String>,
    #[serde(default)]
    pub is_sidechain: Option<bool>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    pub message: RawUserMessageBody,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAssistantMessage {
    #[serde(default)]
    pub parent_uuid: Option<String>,
    #[serde(default)]
    pub is_sidechain: Option<bool>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    pub message: RawAssistantMessageBody,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSystemEntry {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawProgressData {
    #[serde(rename = "type")]
    #[serde(default)]
    pub data_type: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawProgressEntry {
    #[serde(default)]
    pub parent_uuid: Option<String>,
    #[serde(default)]
    pub data: Option<RawProgressData>,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

// ── Unified entry ──

#[derive(Debug, Clone)]
pub enum RawEntry {
    User(RawUserMessage),
    Assistant(RawAssistantMessage),
    System(RawSystemEntry),
    Progress(RawProgressEntry),
    Other, // file-history-snapshot, queue-operation, etc.
}

pub fn parse_jsonl_line(line: &str) -> Option<RawEntry> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    // First parse as a generic Value to check the "type" field
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let entry_type = value.get("type")?.as_str()?;

    match entry_type {
        "user" => {
            let msg: RawUserMessage = serde_json::from_value(value).ok()?;
            Some(RawEntry::User(msg))
        }
        "assistant" => {
            let msg: RawAssistantMessage = serde_json::from_value(value).ok()?;
            Some(RawEntry::Assistant(msg))
        }
        "system" => {
            let entry: RawSystemEntry = serde_json::from_value(value).ok()?;
            Some(RawEntry::System(entry))
        }
        "progress" => {
            let entry: RawProgressEntry = serde_json::from_value(value).ok()?;
            Some(RawEntry::Progress(entry))
        }
        _ => Some(RawEntry::Other),
    }
}

pub struct ParseResult {
    pub entries: Vec<RawEntry>,
    pub remainder: String,
}

pub fn parse_jsonl_chunk(chunk: &str) -> ParseResult {
    let mut entries = Vec::new();
    let mut remainder = String::new();

    let lines: Vec<&str> = chunk.split('\n').collect();

    for (i, line) in lines.iter().enumerate() {
        // If last line and chunk doesn't end with newline, it's a partial line
        if i == lines.len() - 1 && !chunk.ends_with('\n') {
            remainder = line.to_string();
            continue;
        }
        if let Some(entry) = parse_jsonl_line(line) {
            entries.push(entry);
        }
    }

    ParseResult { entries, remainder }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_user_message() {
        let line = r#"{"type":"user","message":{"role":"user","content":"hello"},"uuid":"u1","timestamp":"2025-01-01T00:00:00Z","sessionId":"s1","cwd":"/tmp","version":"1.0"}"#;
        let entry = parse_jsonl_line(line).unwrap();
        match entry {
            RawEntry::User(msg) => {
                assert_eq!(msg.uuid.as_deref(), Some("u1"));
                assert_eq!(msg.session_id.as_deref(), Some("s1"));
            }
            _ => panic!("Expected User entry"),
        }
    }

    #[test]
    fn test_parse_assistant_message() {
        let line = r#"{"type":"assistant","message":{"model":"claude-sonnet-4-20250514","content":[{"type":"text","text":"hi"}]},"uuid":"a1","timestamp":"2025-01-01T00:00:00Z","sessionId":"s1"}"#;
        let entry = parse_jsonl_line(line).unwrap();
        match entry {
            RawEntry::Assistant(msg) => {
                assert_eq!(msg.message.model.as_deref(), Some("claude-sonnet-4-20250514"));
                assert_eq!(msg.message.content.len(), 1);
            }
            _ => panic!("Expected Assistant entry"),
        }
    }

    #[test]
    fn test_parse_system_entry() {
        let line = r#"{"type":"system","subtype":"turn_duration","durationMs":1500,"timestamp":"2025-01-01T00:00:00Z"}"#;
        let entry = parse_jsonl_line(line).unwrap();
        match entry {
            RawEntry::System(sys) => {
                assert_eq!(sys.subtype.as_deref(), Some("turn_duration"));
                assert_eq!(sys.duration_ms, Some(1500));
            }
            _ => panic!("Expected System entry"),
        }
    }

    #[test]
    fn test_parse_chunk_with_remainder() {
        let chunk = "line1\nline2\npartial";
        let result = parse_jsonl_chunk(chunk);
        assert_eq!(result.remainder, "partial");
    }

    #[test]
    fn test_parse_chunk_complete() {
        let chunk = r#"{"type":"system","subtype":"turn_duration","durationMs":100}
"#;
        let result = parse_jsonl_chunk(chunk);
        assert_eq!(result.entries.len(), 1);
        assert!(result.remainder.is_empty());
    }

    #[test]
    fn test_parse_unknown_type() {
        let line = r#"{"type":"file-history-snapshot","messageId":"m1","snapshot":{}}"#;
        let entry = parse_jsonl_line(line).unwrap();
        assert!(matches!(entry, RawEntry::Other));
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(parse_jsonl_line("").is_none());
        assert!(parse_jsonl_line("   ").is_none());
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_jsonl_line("not json").is_none());
    }
}

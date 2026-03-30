use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct JsonlLine {
    #[serde(rename = "type")]
    pub line_type: String,
    pub subtype: Option<String>,
    pub message: Option<Message>,
    pub timestamp: Option<String>,
    pub uuid: Option<String>,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    pub snapshot: Option<serde_json::Value>,
    pub operation: Option<String>,
    pub content: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub role: Option<String>,
    pub content: Option<Vec<serde_json::Value>>,
    pub usage: Option<Usage>,
    pub model: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ContentBlock {
    Thinking { thinking: String },
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: Option<serde_json::Value>, is_error: Option<bool> },
}

impl ContentBlock {
    pub fn from_value(v: &serde_json::Value) -> Option<ContentBlock> {
        let t = v.get("type")?.as_str()?;
        match t {
            "thinking" => Some(ContentBlock::Thinking {
                thinking: v.get("thinking")?.as_str()?.to_string(),
            }),
            "text" => Some(ContentBlock::Text {
                text: v.get("text")?.as_str()?.to_string(),
            }),
            "tool_use" => Some(ContentBlock::ToolUse {
                id: v.get("id")?.as_str()?.to_string(),
                name: v.get("name")?.as_str()?.to_string(),
                input: v.get("input")?.clone(),
            }),
            "tool_result" => Some(ContentBlock::ToolResult {
                tool_use_id: v.get("tool_use_id")?.as_str()?.to_string(),
                content: v.get("content").cloned(),
                is_error: v.get("is_error").and_then(|v| v.as_bool()),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct WasteFinding {
    pub category: String,
    pub description: String,
    pub estimated_tokens: u64,
    pub details: Vec<String>,
}

#[derive(Debug)]
pub struct SessionInfo {
    pub path: String,
    pub project_name: String,
    pub is_subagent: bool,
    pub parent_session: Option<String>,
}

#[derive(Debug)]
pub struct Session {
    pub info: SessionInfo,
    pub lines: Vec<JsonlLine>,
    pub total_tokens: u64,
    pub subagents: Vec<Session>,
}

#[derive(Debug)]
pub struct Report {
    pub date: String,
    pub sessions: Vec<SessionReport>,
}

#[derive(Debug)]
pub struct SessionReport {
    pub project_name: String,
    pub total_tokens: u64,
    pub wasted_tokens: u64,
    pub waste_ratio: f64,
    pub findings: Vec<WasteFinding>,
    pub subagent_count: usize,
}

impl Report {
    pub fn total_tokens(&self) -> u64 {
        self.sessions.iter().map(|s| s.total_tokens).sum()
    }
    pub fn total_wasted(&self) -> u64 {
        self.sessions.iter().map(|s| s.wasted_tokens).sum()
    }
    pub fn waste_ratio(&self) -> f64 {
        let total = self.total_tokens();
        if total == 0 {
            return 0.0;
        }
        self.total_wasted() as f64 / total as f64
    }
    pub fn category_totals(&self) -> Vec<(String, u64)> {
        let mut map: HashMap<String, u64> = HashMap::new();
        for session in &self.sessions {
            for finding in &session.findings {
                *map.entry(finding.category.clone()).or_default() += finding.estimated_tokens;
            }
        }
        let mut sorted: Vec<_> = map.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted
    }
}

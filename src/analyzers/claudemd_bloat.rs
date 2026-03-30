use super::WasteAnalyzer;
use crate::types::{Session, WasteFinding};

pub struct ClaudeMdBloatAnalyzer;

impl WasteAnalyzer for ClaudeMdBloatAnalyzer {
    fn name(&self) -> &str {
        "CLAUDE.md bloat"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        // Find the first system message (not a subtype like stop_hook_summary)
        for line in &session.lines {
            if line.line_type != "system" {
                continue;
            }
            // Skip lines with subtypes (e.g., stop_hook_summary)
            if line.subtype.is_some() {
                continue;
            }

            // Check content field size (from the line-level content, or from message.content)
            let content_size = if let Some(ref msg) = line.message {
                if let Some(ref content) = msg.content {
                    serde_json::to_string(content)
                        .map(|s| s.len() as u64)
                        .unwrap_or(0)
                } else {
                    0
                }
            } else if let Some(ref content) = line.content {
                content.to_string().len() as u64
            } else {
                0
            };

            if content_size > 40000 {
                let estimated_tokens = content_size / 4;
                return vec![WasteFinding {
                    category: "CLAUDE.md bloat".to_string(),
                    description: format!(
                        "System message (CLAUDE.md) is {} bytes (~{} tokens) — consider trimming",
                        content_size, estimated_tokens
                    ),
                    estimated_tokens,
                    details: vec![format!(
                        "Content loaded on every session start. Bytes: {}",
                        content_size
                    )],
                }];
            }

            // Only check the first system message
            break;
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn make_session(lines: Vec<JsonlLine>) -> Session {
        Session {
            info: SessionInfo {
                path: "test".to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines,
            total_tokens: 0,
            subagents: vec![],
        }
    }

    #[test]
    fn test_detects_large_system_message() {
        let big_content = "x".repeat(50000);
        let line = JsonlLine {
            line_type: "system".to_string(),
            subtype: None,
            message: None,
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: Some(serde_json::json!(big_content)),
        };
        let session = make_session(vec![line]);
        let findings = ClaudeMdBloatAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "CLAUDE.md bloat");
        assert!(findings[0].estimated_tokens > 10000);
    }

    #[test]
    fn test_ignores_small_system_message() {
        let line = JsonlLine {
            line_type: "system".to_string(),
            subtype: None,
            message: None,
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: Some(serde_json::json!("small content")),
        };
        let session = make_session(vec![line]);
        let findings = ClaudeMdBloatAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_skips_stop_hook_summary() {
        let big_content = "x".repeat(50000);
        let lines = vec![
            JsonlLine {
                line_type: "system".to_string(),
                subtype: Some("stop_hook_summary".to_string()),
                message: None,
                timestamp: None,
                uuid: None,
                message_id: None,
                snapshot: None,
                operation: None,
                content: Some(serde_json::json!(big_content)),
            },
            JsonlLine {
                line_type: "system".to_string(),
                subtype: None,
                message: None,
                timestamp: None,
                uuid: None,
                message_id: None,
                snapshot: None,
                operation: None,
                content: Some(serde_json::json!("small")),
            },
        ];
        let session = make_session(lines);
        let findings = ClaudeMdBloatAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }
}

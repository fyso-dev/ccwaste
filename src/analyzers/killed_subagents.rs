use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};

pub struct KilledSubagentsAnalyzer;

impl KilledSubagentsAnalyzer {
    fn is_killed(session: &Session) -> bool {
        // A subagent is "killed" if its last assistant message contains a tool_use but no text
        let last_assistant = session
            .lines
            .iter()
            .rev()
            .find(|l| l.line_type == "assistant");

        let Some(line) = last_assistant else {
            return false;
        };
        let Some(ref msg) = line.message else {
            return false;
        };
        let Some(ref content) = msg.content else {
            return false;
        };

        let mut has_tool_use = false;
        let mut has_text = false;

        for val in content {
            if let Some(block) = ContentBlock::from_value(val) {
                match block {
                    ContentBlock::ToolUse { .. } => has_tool_use = true,
                    ContentBlock::Text { text } => {
                        if !text.trim().is_empty() {
                            has_text = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        has_tool_use && !has_text
    }
}

impl WasteAnalyzer for KilledSubagentsAnalyzer {
    fn name(&self) -> &str {
        "Killed subagents"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut findings = vec![];

        for subagent in &session.subagents {
            if Self::is_killed(subagent) {
                findings.push(WasteFinding {
                    category: "Killed subagents".to_string(),
                    description: format!(
                        "Subagent '{}' was killed mid-tool-call ({} tokens wasted)",
                        subagent.info.project_name, subagent.total_tokens
                    ),
                    estimated_tokens: subagent.total_tokens,
                    details: vec![format!("path: {}", subagent.info.path)],
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{JsonlLine, Message, SessionInfo, Usage};

    fn make_session(subagents: Vec<Session>) -> Session {
        Session {
            info: SessionInfo {
                path: "test".to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines: vec![],
            total_tokens: 1000,
            subagents,
        }
    }

    fn make_subagent(content: Vec<serde_json::Value>, total_tokens: u64) -> Session {
        Session {
            info: SessionInfo {
                path: "/tmp/subagent.jsonl".to_string(),
                project_name: "sub1".to_string(),
                is_subagent: true,
                parent_session: Some("parent".to_string()),
            },
            lines: vec![JsonlLine {
                line_type: "assistant".to_string(),
                subtype: None,
                message: Some(Message {
                    id: Some("msg_1".to_string()),
                    role: Some("assistant".to_string()),
                    content: Some(content),
                    usage: Some(Usage {
                        input_tokens: Some(total_tokens / 2),
                        output_tokens: Some(total_tokens / 2),
                        cache_read_input_tokens: None,
                        cache_creation_input_tokens: None,
                    }),
                    model: None,
                }),
                timestamp: None,
                uuid: None,
                message_id: None,
                snapshot: None,
                operation: None,
                content: None,
            }],
            total_tokens,
            subagents: vec![],
        }
    }

    #[test]
    fn test_killed_subagent_detected() {
        let content = vec![serde_json::json!({
            "type": "tool_use",
            "id": "tu_1",
            "name": "Bash",
            "input": {"command": "ls"}
        })];
        let sub = make_subagent(content, 5000);
        let session = make_session(vec![sub]);
        let findings = KilledSubagentsAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "Killed subagents");
        assert_eq!(findings[0].estimated_tokens, 5000);
    }

    #[test]
    fn test_normal_subagent_not_flagged() {
        let content = vec![
            serde_json::json!({
                "type": "text",
                "text": "Done! Here are the results."
            }),
            serde_json::json!({
                "type": "tool_use",
                "id": "tu_1",
                "name": "Bash",
                "input": {"command": "ls"}
            }),
        ];
        let sub = make_subagent(content, 5000);
        let session = make_session(vec![sub]);
        let findings = KilledSubagentsAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn test_no_subagents() {
        let session = make_session(vec![]);
        let findings = KilledSubagentsAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 0);
    }
}

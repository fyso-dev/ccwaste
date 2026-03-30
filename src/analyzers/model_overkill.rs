use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};
use std::collections::HashMap;

pub struct ModelOverkillAnalyzer;

const SIMPLE_TOOLS: &[&str] = &["Read", "Glob", "Bash", "LS"];

fn is_simple_bash(input: &serde_json::Value) -> bool {
    if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
        let trimmed = cmd.trim();
        let simple_prefixes = [
            "ls", "cat", "pwd", "echo", "which", "file", "wc", "head", "tail", "mkdir", "touch",
            "rm ", "cp ", "mv ",
        ];
        simple_prefixes.iter().any(|p| trimmed.starts_with(p)) || !trimmed.contains("&&")
    } else {
        true
    }
}

impl WasteAnalyzer for ModelOverkillAnalyzer {
    fn name(&self) -> &str {
        "model_overkill"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut overkill_turns = 0u64;
        let mut total_waste_tokens = 0u64;
        let mut tool_turn_counts: HashMap<String, u64> = HashMap::new();

        for line in &session.lines {
            if line.line_type != "assistant" {
                continue;
            }
            let Some(ref msg) = line.message else {
                continue;
            };
            let is_opus = msg
                .model
                .as_deref()
                .map(|m| m.to_lowercase().contains("opus"))
                .unwrap_or(false);
            if !is_opus {
                continue;
            }
            let Some(ref content) = msg.content else {
                continue;
            };

            let blocks: Vec<ContentBlock> = content
                .iter()
                .filter_map(|v| ContentBlock::from_value(v))
                .collect();

            if blocks.is_empty() {
                continue;
            }

            let all_simple_tool_use = blocks.iter().all(|b| match b {
                ContentBlock::ToolUse { name, input, .. } => {
                    if name == "Bash" {
                        is_simple_bash(input)
                    } else {
                        SIMPLE_TOOLS.contains(&name.as_str())
                    }
                }
                _ => false,
            });

            if all_simple_tool_use {
                overkill_turns += 1;
                let input_tokens = msg
                    .usage
                    .as_ref()
                    .and_then(|u| u.input_tokens)
                    .unwrap_or(0);
                let waste = input_tokens * 4 / 5;
                total_waste_tokens += waste;
                for b in &blocks {
                    if let ContentBlock::ToolUse { name, .. } = b {
                        *tool_turn_counts.entry(name.clone()).or_default() += 1;
                    }
                }
            }
        }

        if overkill_turns == 0 {
            return vec![];
        }

        let mut sorted_tools: Vec<_> = tool_turn_counts.into_iter().collect();
        sorted_tools.sort_by(|a, b| b.1.cmp(&a.1));
        let details = sorted_tools
            .iter()
            .map(|(name, count)| format!("{}: {} turns", name, count))
            .collect();

        vec![WasteFinding {
            category: "model_overkill".to_string(),
            description: format!(
                "{} opus turns used for simple tool calls",
                overkill_turns
            ),
            estimated_tokens: total_waste_tokens,
            details,
        }]
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
    fn test_detects_opus_simple_tool_use() {
        let line = JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some("msg_1".to_string()),
                role: Some("assistant".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_use",
                    "id": "tu_1",
                    "name": "Read",
                    "input": {"file_path": "/tmp/foo.rs"}
                })]),
                usage: Some(Usage {
                    input_tokens: Some(1000),
                    output_tokens: Some(50),
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                }),
                model: Some("claude-opus-4-20250514".to_string()),
            }),
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: None,
        };
        let session = make_session(vec![line]);
        let findings = ModelOverkillAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "model_overkill");
        assert_eq!(findings[0].estimated_tokens, 800); // 4/5 of 1000
        assert!(findings[0].details.iter().any(|d| d.contains("Read: 1 turns")));
    }

    #[test]
    fn test_ignores_opus_with_thinking() {
        let line = JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some("msg_1".to_string()),
                role: Some("assistant".to_string()),
                content: Some(vec![
                    serde_json::json!({"type": "thinking", "thinking": "Let me think..."}),
                    serde_json::json!({"type": "tool_use", "id": "tu_1", "name": "Read", "input": {"file_path": "/tmp/foo.rs"}}),
                ]),
                usage: Some(Usage {
                    input_tokens: Some(1000),
                    output_tokens: Some(50),
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                }),
                model: Some("claude-opus-4-20250514".to_string()),
            }),
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: None,
        };
        let session = make_session(vec![line]);
        let findings = ModelOverkillAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_ignores_non_opus() {
        let line = JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some("msg_1".to_string()),
                role: Some("assistant".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_use",
                    "id": "tu_1",
                    "name": "Read",
                    "input": {"file_path": "/tmp/foo.rs"}
                })]),
                usage: Some(Usage {
                    input_tokens: Some(1000),
                    output_tokens: Some(50),
                    cache_read_input_tokens: None,
                    cache_creation_input_tokens: None,
                }),
                model: Some("claude-sonnet-4-20250514".to_string()),
            }),
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: None,
        };
        let session = make_session(vec![line]);
        let findings = ModelOverkillAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }
}

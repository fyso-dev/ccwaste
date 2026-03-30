use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};

pub struct ModelOverkillAnalyzer;

const SIMPLE_TOOLS: &[&str] = &["Read", "Glob", "Bash", "LS"];

fn is_simple_bash(input: &serde_json::Value) -> bool {
    if let Some(cmd) = input.get("command").and_then(|v| v.as_str()) {
        let trimmed = cmd.trim();
        // Simple commands: ls, cat, pwd, echo, which, file, wc, head, tail, etc.
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
        let mut details = vec![];

        for line in &session.lines {
            if line.line_type != "assistant" {
                continue;
            }
            let Some(ref msg) = line.message else {
                continue;
            };
            // Check if model contains "opus" (case-insensitive)
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

            // Check if message ONLY contains tool_use blocks for simple tools
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
                // Waste = 4/5 of input_tokens (the overpay portion)
                let waste = input_tokens * 4 / 5;
                total_waste_tokens += waste;
                let tool_names: Vec<String> = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::ToolUse { name, .. } => Some(name.clone()),
                        _ => None,
                    })
                    .collect();
                details.push(format!(
                    "opus used for simple tool call(s): {} (~{} tokens wasted)",
                    tool_names.join(", "),
                    waste
                ));
            }
        }

        if overkill_turns == 0 {
            return vec![];
        }

        vec![WasteFinding {
            category: "model_overkill".to_string(),
            description: format!(
                "{} turns used opus for simple tool calls, wasting ~{} tokens",
                overkill_turns, total_waste_tokens
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

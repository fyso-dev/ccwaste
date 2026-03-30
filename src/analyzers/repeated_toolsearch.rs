use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};
use std::collections::HashMap;

pub struct RepeatedToolSearchAnalyzer;

impl WasteAnalyzer for RepeatedToolSearchAnalyzer {
    fn name(&self) -> &str {
        "Repeated ToolSearch"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        // Step 1: Find ToolSearch tool_use calls, group by query
        let mut query_calls: HashMap<String, Vec<String>> = HashMap::new(); // query -> [tool_use_id]
        let mut tool_ids: HashMap<String, String> = HashMap::new(); // tool_use_id -> query

        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolUse { id, name, input }) = ContentBlock::from_value(val) {
                    if name == "ToolSearch" {
                        if let Some(query) = input.get("query").and_then(|v| v.as_str()) {
                            let q = query.to_string();
                            query_calls.entry(q.clone()).or_default().push(id.clone());
                            tool_ids.insert(id, q);
                        }
                    }
                }
            }
        }

        // Step 2: Build tool_use_id -> result_bytes map
        let mut result_bytes: HashMap<String, u64> = HashMap::new();
        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    ..
                }) = ContentBlock::from_value(val)
                {
                    if tool_ids.contains_key(&tool_use_id) {
                        let bytes = content
                            .map(|c| c.to_string().len() as u64)
                            .unwrap_or(0);
                        result_bytes.insert(tool_use_id, bytes);
                    }
                }
            }
        }

        // Step 3: Flag queries repeated > 1 time
        let mut findings = vec![];
        for (query, ids) in &query_calls {
            let count = ids.len() as u64;
            if count <= 1 {
                continue;
            }
            let total_bytes: u64 = ids
                .iter()
                .filter_map(|id| result_bytes.get(id))
                .sum();
            let avg_bytes = if ids.is_empty() {
                0
            } else {
                total_bytes / ids.len() as u64
            };
            let waste = (count - 1) * avg_bytes / 4;

            findings.push(WasteFinding {
                category: "Repeated ToolSearch".to_string(),
                description: format!(
                    "ToolSearch '{}' repeated {} times ({} excess)",
                    query,
                    count,
                    count - 1
                ),
                estimated_tokens: waste,
                details: vec![format!(
                    "avg result: {} bytes, wasted: ~{} tokens",
                    avg_bytes, waste
                )],
            });
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{JsonlLine, Message, Session, SessionInfo, Usage};

    fn make_toolsearch_line(id: &str, query: &str) -> JsonlLine {
        JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some(format!("msg_{}", id)),
                role: Some("assistant".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_use",
                    "id": id,
                    "name": "ToolSearch",
                    "input": {"query": query}
                })]),
                usage: Some(Usage {
                    input_tokens: Some(50),
                    output_tokens: Some(10),
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
        }
    }

    fn make_result_line(tool_use_id: &str, content: &str) -> JsonlLine {
        JsonlLine {
            line_type: "human".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some(format!("msg_r_{}", tool_use_id)),
                role: Some("user".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": content
                })]),
                usage: None,
                model: None,
            }),
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: None,
        }
    }

    #[test]
    fn test_repeated_toolsearch_detected() {
        let session = Session {
            info: SessionInfo {
                path: "test".to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines: vec![
                make_toolsearch_line("ts_1", "select:Read,Edit"),
                make_result_line("ts_1", "Found Read tool definition with 200 bytes of schema"),
                make_toolsearch_line("ts_2", "select:Read,Edit"),
                make_result_line("ts_2", "Found Read tool definition with 200 bytes of schema"),
                make_toolsearch_line("ts_3", "notebook jupyter"),
                make_result_line("ts_3", "Found notebook tool"),
            ],
            total_tokens: 5000,
            subagents: vec![],
        };

        let findings = RepeatedToolSearchAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.category, "Repeated ToolSearch");
        assert!(f.description.contains("select:Read,Edit"));
        assert!(f.description.contains("2 times"));
        assert!(f.estimated_tokens > 0);
    }

    #[test]
    fn test_no_repeated_toolsearch() {
        let session = Session {
            info: SessionInfo {
                path: "test".to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines: vec![
                make_toolsearch_line("ts_1", "select:Read"),
                make_result_line("ts_1", "schema data"),
                make_toolsearch_line("ts_2", "select:Edit"),
                make_result_line("ts_2", "schema data"),
            ],
            total_tokens: 5000,
            subagents: vec![],
        };

        let findings = RepeatedToolSearchAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 0);
    }
}

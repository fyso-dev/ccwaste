use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};

pub struct BroadSearchesAnalyzer;

fn is_broad_path(path: Option<&str>) -> bool {
    match path {
        None => true,
        Some(p) => {
            let trimmed = p.trim();
            trimmed.is_empty()
                || trimmed == "/"
                || trimmed == "."
                || trimmed == "./"
                || trimmed.len() < 10
        }
    }
}

impl WasteAnalyzer for BroadSearchesAnalyzer {
    fn name(&self) -> &str {
        "broad_searches"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut broad_count = 0u64;
        let mut broad_tool_ids: Vec<String> = vec![];
        let mut details = vec![];

        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolUse { id, name, input }) = ContentBlock::from_value(val) {
                    if name == "Grep" || name == "Glob" {
                        let path = input.get("path").and_then(|v| v.as_str());
                        if is_broad_path(path) {
                            broad_count += 1;
                            broad_tool_ids.push(id.clone());
                            details.push(format!(
                                "{} with path={:?}",
                                name,
                                path.unwrap_or("<none>")
                            ));
                        }
                    }
                }
            }
        }

        if broad_count == 0 {
            return vec![];
        }

        // Estimate waste from result sizes of broad searches
        let mut result_bytes = 0u64;
        let mut junk_path_count = 0u64;
        let junk_dirs = [
            "node_modules/",
            ".git/",
            "dist/",
            "build/",
            "target/",
            ".next/",
            "__pycache__/",
            ".venv/",
        ];

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
                    content: result_content,
                    ..
                }) = ContentBlock::from_value(val)
                {
                    if broad_tool_ids.contains(&tool_use_id) {
                        if let Some(ref c) = result_content {
                            let s = c.to_string();
                            result_bytes += s.len() as u64;
                            for junk in &junk_dirs {
                                if s.contains(junk) {
                                    junk_path_count += 1;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Estimate: broad search results are ~50% wasted
        let estimated_tokens = result_bytes / 8; // half of bytes/4

        if junk_path_count > 0 {
            details.push(format!(
                "{} results contained junk directory paths (node_modules, .git, etc.)",
                junk_path_count
            ));
        }

        vec![WasteFinding {
            category: "broad_searches".to_string(),
            description: format!(
                "{} broad Grep/Glob searches (no specific path), ~{} tokens of results",
                broad_count, estimated_tokens
            ),
            estimated_tokens,
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
    fn test_detects_broad_grep() {
        let lines = vec![
            JsonlLine {
                line_type: "assistant".to_string(),
                subtype: None,
                message: Some(Message {
                    id: Some("msg_1".to_string()),
                    role: Some("assistant".to_string()),
                    content: Some(vec![serde_json::json!({
                        "type": "tool_use",
                        "id": "tu_1",
                        "name": "Grep",
                        "input": {"pattern": "TODO"}
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
            },
            JsonlLine {
                line_type: "human".to_string(),
                subtype: None,
                message: Some(Message {
                    id: None,
                    role: Some("user".to_string()),
                    content: Some(vec![serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": "tu_1",
                        "content": "node_modules/foo/bar.js:1:TODO\nsrc/main.rs:5:TODO"
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
            },
        ];
        let session = make_session(lines);
        let findings = BroadSearchesAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "broad_searches");
        assert!(findings[0].details.iter().any(|d| d.contains("junk")));
    }

    #[test]
    fn test_ignores_specific_path() {
        let lines = vec![JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some("msg_1".to_string()),
                role: Some("assistant".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_use",
                    "id": "tu_1",
                    "name": "Grep",
                    "input": {"pattern": "TODO", "path": "/Users/user/project/src/analyzers"}
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
        }];
        let session = make_session(lines);
        let findings = BroadSearchesAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }
}

use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};

pub struct MissingClaudeignoreAnalyzer;

const JUNK_DIRS: &[&str] = &[
    "node_modules/",
    ".git/",
    "dist/",
    "build/",
    "target/",
    ".next/",
    "__pycache__/",
    ".venv/",
];

impl WasteAnalyzer for MissingClaudeignoreAnalyzer {
    fn name(&self) -> &str {
        "Missing .claudeignore"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut junk_line_count = 0u64;
        let mut junk_dir_hits: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolResult {
                    content: result_content,
                    ..
                }) = ContentBlock::from_value(val)
                {
                    let Some(ref c) = result_content else {
                        continue;
                    };
                    // Get the string content, handling both raw strings and JSON-serialized values
                    let text = if let Some(s) = c.as_str() {
                        s.to_string()
                    } else {
                        c.to_string()
                    };
                    for result_line in text.lines() {
                        for junk in JUNK_DIRS {
                            if result_line.contains(junk) {
                                junk_line_count += 1;
                                *junk_dir_hits.entry(junk.to_string()).or_default() += 1;
                                break; // Only count once per line
                            }
                        }
                    }
                }
            }
        }

        if junk_line_count == 0 {
            return vec![];
        }

        let estimated_tokens = junk_line_count * 50;

        let mut details: Vec<String> = junk_dir_hits
            .iter()
            .map(|(dir, count)| format!("{}: {} result lines", dir, count))
            .collect();
        details.sort();

        vec![WasteFinding {
            category: "Missing .claudeignore".to_string(),
            description: format!(
                "{} tool result lines include paths from ignorable directories (~{} tokens)",
                junk_line_count, estimated_tokens
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
    fn test_detects_junk_paths() {
        let lines = vec![JsonlLine {
            line_type: "human".to_string(),
            subtype: None,
            message: Some(Message {
                id: None,
                role: Some("user".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": "tu_1",
                    "content": "node_modules/express/index.js\nsrc/main.rs\n.git/config\ntarget/debug/build/foo"
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
        let findings = MissingClaudeignoreAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "Missing .claudeignore");
        // 3 junk lines: node_modules, .git, target
        assert_eq!(findings[0].estimated_tokens, 150); // 3 * 50
    }

    #[test]
    fn test_no_junk_paths() {
        let lines = vec![JsonlLine {
            line_type: "human".to_string(),
            subtype: None,
            message: Some(Message {
                id: None,
                role: Some("user".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": "tu_1",
                    "content": "src/main.rs\nsrc/lib.rs"
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
        let findings = MissingClaudeignoreAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }
}

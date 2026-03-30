use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};
use std::collections::HashMap;

pub struct ToolErrorsAnalyzer;

impl WasteAnalyzer for ToolErrorsAnalyzer {
    fn name(&self) -> &str {
        "tool_errors"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        // Step 1: Build tool_use_id -> tool_name lookup
        let mut tool_names: HashMap<String, String> = HashMap::new();
        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolUse { id, name, .. }) = ContentBlock::from_value(val)
                {
                    tool_names.insert(id, name);
                }
            }
        }

        // Step 2: Find error tool_results
        let mut errors_by_tool: HashMap<String, (u64, u64)> = HashMap::new(); // tool_name -> (count, bytes)

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
                    is_error,
                }) = ContentBlock::from_value(val)
                {
                    let content_str = content
                        .as_ref()
                        .map(|c| c.to_string())
                        .unwrap_or_default();
                    let is_err = is_error.unwrap_or(false)
                        || content_str.contains("<tool_use_error>")
                        || content_str.contains("Exit code 1");

                    if !is_err {
                        continue;
                    }

                    let tool_name = tool_names
                        .get(&tool_use_id)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());
                    let entry = errors_by_tool.entry(tool_name).or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += content_str.len() as u64;
                }
            }
        }

        if errors_by_tool.is_empty() {
            return vec![];
        }

        let total_bytes: u64 = errors_by_tool.values().map(|(_, b)| b).sum();
        let estimated_tokens = total_bytes / 4;

        let mut details: Vec<String> = errors_by_tool
            .iter()
            .map(|(tool, (count, bytes))| {
                format!("{}: {} errors, ~{} bytes", tool, count, bytes)
            })
            .collect();
        details.sort();

        let total_errors: u64 = errors_by_tool.values().map(|(c, _)| c).sum();

        vec![WasteFinding {
            category: "tool_errors".to_string(),
            description: format!(
                "{} tool errors consuming ~{} estimated tokens",
                total_errors, estimated_tokens
            ),
            estimated_tokens,
            details,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{build_session, parse_jsonl_file};
    use crate::types::SessionInfo;
    use std::path::PathBuf;

    #[test]
    fn test_tool_errors_analyzer() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with_errors.jsonl");
        let lines = parse_jsonl_file(&path).unwrap();
        let session = build_session(
            SessionInfo {
                path: path.to_string_lossy().to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines,
        );
        let findings = ToolErrorsAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.category, "tool_errors");
        assert!(f.estimated_tokens > 0);
        // Should have Bash and Glob errors
        assert!(f.details.iter().any(|d| d.starts_with("Bash:")));
        assert!(f.details.iter().any(|d| d.starts_with("Glob:")));
    }
}

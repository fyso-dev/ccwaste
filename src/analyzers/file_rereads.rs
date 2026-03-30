use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};
use std::collections::HashMap;

pub struct FileRereadsAnalyzer;

impl WasteAnalyzer for FileRereadsAnalyzer {
    fn name(&self) -> &str {
        "File re-reads"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        // Step 1: Find all Read tool_use calls and collect tool_use_id -> file_path
        let mut read_calls: HashMap<String, String> = HashMap::new(); // tool_use_id -> file_path
        let mut file_read_counts: HashMap<String, u64> = HashMap::new(); // file_path -> count
        let mut file_read_ids: HashMap<String, Vec<String>> = HashMap::new(); // file_path -> [tool_use_id]

        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolUse { id, name, input }) = ContentBlock::from_value(val) {
                    if name == "Read" {
                        if let Some(file_path) = input.get("file_path").and_then(|v| v.as_str()) {
                            let fp = file_path.to_string();
                            read_calls.insert(id.clone(), fp.clone());
                            *file_read_counts.entry(fp.clone()).or_default() += 1;
                            file_read_ids.entry(fp).or_default().push(id);
                        }
                    }
                }
            }
        }

        // Step 2: Build tool_use_id -> result_bytes map from tool_result blocks
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
                    if read_calls.contains_key(&tool_use_id) {
                        let bytes = content
                            .map(|c| c.to_string().len() as u64)
                            .unwrap_or(0);
                        result_bytes.insert(tool_use_id, bytes);
                    }
                }
            }
        }

        // Step 3: Flag files read more than 2 times, aggregate into one finding
        let mut details = vec![];
        let mut total_waste_tokens = 0u64;

        let mut flagged: Vec<_> = file_read_counts
            .iter()
            .filter(|(_, c)| **c > 2)
            .collect();
        flagged.sort_by(|a, b| b.1.cmp(a.1));

        for (file_path, count) in &flagged {
            let ids = file_read_ids.get(*file_path).unwrap();
            let total_result_bytes: u64 = ids
                .iter()
                .filter_map(|id| result_bytes.get(id))
                .sum();
            let avg_bytes = if ids.is_empty() {
                0
            } else {
                total_result_bytes / ids.len() as u64
            };
            let waste_tokens = (*count - 1) * avg_bytes / 4;
            total_waste_tokens += waste_tokens;

            let short_name = file_path
                .rsplit('/')
                .next()
                .unwrap_or(file_path);
            details.push(format!(
                "{} x{} (~{}K wasted)",
                short_name,
                count,
                waste_tokens / 1000
            ));
        }

        if flagged.is_empty() {
            return vec![];
        }

        vec![WasteFinding {
            category: "File re-reads".to_string(),
            description: format!(
                "{} files read more than twice",
                flagged.len()
            ),
            estimated_tokens: total_waste_tokens,
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
    fn test_file_rereads_analyzer() {
        let path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with_rereads.jsonl");
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
        let findings = FileRereadsAnalyzer.analyze(&session);
        // Returns ONE aggregated finding for all files read > 2 times
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.category, "File re-reads");
        assert!(f.description.contains("1 files read more than twice"));
        // Details should contain foo.rs
        assert!(f.details.iter().any(|d| d.contains("foo.rs")));
        assert!(f.estimated_tokens > 0);
    }
}

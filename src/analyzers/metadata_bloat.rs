use super::WasteAnalyzer;
use crate::types::{Session, WasteFinding};

pub struct MetadataBloatAnalyzer;

impl WasteAnalyzer for MetadataBloatAnalyzer {
    fn name(&self) -> &str {
        "Metadata bloat"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut file_history_count: u64 = 0;
        let mut file_history_bytes: u64 = 0;
        let mut queue_op_count: u64 = 0;
        let mut queue_op_bytes: u64 = 0;
        let mut stop_hook_count: u64 = 0;
        let mut stop_hook_bytes: u64 = 0;

        for line in &session.lines {
            let line_type = line.line_type.as_str();
            let subtype = line.subtype.as_deref().unwrap_or("");

            if line_type == "file-history-snapshot" {
                file_history_count += 1;
                if let Some(ref snapshot) = line.snapshot {
                    file_history_bytes += snapshot.to_string().len() as u64;
                }
            } else if line_type == "queue-operation" {
                queue_op_count += 1;
                if let Some(ref content) = line.content {
                    queue_op_bytes += content.to_string().len() as u64;
                }
            } else if subtype == "stop_hook_summary" {
                stop_hook_count += 1;
                if let Some(ref content) = line.content {
                    stop_hook_bytes += content.to_string().len() as u64;
                }
            }
        }

        let total_count = file_history_count + queue_op_count + stop_hook_count;
        if total_count == 0 {
            return vec![];
        }

        let total_bytes = file_history_bytes + queue_op_bytes + stop_hook_bytes;
        let estimated_tokens = total_bytes / 4;

        let mut details = vec![];
        if file_history_count > 0 {
            details.push(format!(
                "file-history-snapshot: {} occurrences, ~{} bytes",
                file_history_count, file_history_bytes
            ));
        }
        if queue_op_count > 0 {
            details.push(format!(
                "queue-operation: {} occurrences, ~{} bytes",
                queue_op_count, queue_op_bytes
            ));
        }
        if stop_hook_count > 0 {
            details.push(format!(
                "stop_hook_summary: {} occurrences, ~{} bytes",
                stop_hook_count, stop_hook_bytes
            ));
        }

        vec![WasteFinding {
            category: "Metadata bloat".to_string(),
            description: format!(
                "{} metadata messages consuming ~{} estimated tokens",
                total_count, estimated_tokens
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
    fn test_metadata_bloat_analyzer() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/with_metadata.jsonl");
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
        let findings = MetadataBloatAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.category, "Metadata bloat");
        assert!(f.estimated_tokens > 0);
        // Should have 3 file-history-snapshot, 2 queue-operation, 2 stop_hook_summary = 7 total
        assert!(f.details.iter().any(|d| d.contains("file-history-snapshot: 3")));
        assert!(f.details.iter().any(|d| d.contains("queue-operation: 2")));
        assert!(f.details.iter().any(|d| d.contains("stop_hook_summary: 2")));
    }
}

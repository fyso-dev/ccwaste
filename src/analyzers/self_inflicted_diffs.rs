use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};

pub struct SelfInflictedDiffsAnalyzer;

impl WasteAnalyzer for SelfInflictedDiffsAnalyzer {
    fn name(&self) -> &str {
        "self_inflicted_diffs"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        // Track indices of Edit/Write tool_use calls
        let mut edit_indices: Vec<usize> = vec![];

        for (i, line) in session.lines.iter().enumerate() {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolUse { name, .. }) = ContentBlock::from_value(val) {
                    if name == "Edit" || name == "Write" {
                        edit_indices.push(i);
                    }
                }
            }
        }

        // Now find file-history-snapshots that appear within 3 messages after an edit
        let mut snapshot_count = 0u64;
        let mut snapshot_bytes = 0u64;

        for (i, line) in session.lines.iter().enumerate() {
            if line.line_type != "file-history-snapshot" {
                continue;
            }
            // Check if any edit happened within 3 messages before this snapshot
            let is_self_inflicted = edit_indices
                .iter()
                .any(|&edit_i| edit_i < i && (i - edit_i) <= 3);

            if is_self_inflicted {
                snapshot_count += 1;
                if let Some(ref snapshot) = line.snapshot {
                    snapshot_bytes += snapshot.to_string().len() as u64;
                }
            }
        }

        if snapshot_count == 0 {
            return vec![];
        }

        let estimated_tokens = snapshot_bytes / 4;

        vec![WasteFinding {
            category: "self_inflicted_diffs".to_string(),
            description: format!(
                "{} file-history-snapshots triggered by Edit/Write (~{} tokens)",
                snapshot_count, estimated_tokens
            ),
            estimated_tokens,
            details: vec![format!(
                "{} snapshot bytes from self-inflicted diffs",
                snapshot_bytes
            )],
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
    fn test_detects_self_inflicted_snapshot() {
        let lines = vec![
            // Edit tool_use at index 0
            JsonlLine {
                line_type: "assistant".to_string(),
                subtype: None,
                message: Some(Message {
                    id: Some("msg_1".to_string()),
                    role: Some("assistant".to_string()),
                    content: Some(vec![serde_json::json!({
                        "type": "tool_use",
                        "id": "tu_1",
                        "name": "Edit",
                        "input": {"file_path": "/tmp/foo.rs", "old_string": "a", "new_string": "b"}
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
            // tool_result at index 1
            JsonlLine {
                line_type: "human".to_string(),
                subtype: None,
                message: Some(Message {
                    id: None,
                    role: Some("user".to_string()),
                    content: Some(vec![serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": "tu_1",
                        "content": "ok"
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
            // file-history-snapshot at index 2 (within 3 of edit at index 0)
            JsonlLine {
                line_type: "file-history-snapshot".to_string(),
                subtype: None,
                message: None,
                timestamp: None,
                uuid: None,
                message_id: None,
                snapshot: Some(serde_json::json!({"file": "/tmp/foo.rs", "content": "updated content here with some data"})),
                operation: None,
                content: None,
            },
        ];
        let session = make_session(lines);
        let findings = SelfInflictedDiffsAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "self_inflicted_diffs");
        assert!(findings[0].estimated_tokens > 0);
    }

    #[test]
    fn test_ignores_distant_snapshot() {
        let mut lines = vec![
            // Edit at index 0
            JsonlLine {
                line_type: "assistant".to_string(),
                subtype: None,
                message: Some(Message {
                    id: Some("msg_1".to_string()),
                    role: Some("assistant".to_string()),
                    content: Some(vec![serde_json::json!({
                        "type": "tool_use",
                        "id": "tu_1",
                        "name": "Edit",
                        "input": {"file_path": "/tmp/foo.rs", "old_string": "a", "new_string": "b"}
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
        // Add 4 filler lines (indices 1-4)
        for _ in 0..4 {
            lines.push(JsonlLine {
                line_type: "human".to_string(),
                subtype: None,
                message: None,
                timestamp: None,
                uuid: None,
                message_id: None,
                snapshot: None,
                operation: None,
                content: None,
            });
        }
        // Snapshot at index 5 (distance > 3 from edit at 0)
        lines.push(JsonlLine {
            line_type: "file-history-snapshot".to_string(),
            subtype: None,
            message: None,
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: Some(serde_json::json!({"content": "data"})),
            operation: None,
            content: None,
        });
        let session = make_session(lines);
        let findings = SelfInflictedDiffsAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }
}

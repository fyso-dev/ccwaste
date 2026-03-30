use super::WasteAnalyzer;
use crate::types::{Session, WasteFinding};
use std::collections::HashSet;

pub struct ContextAccumulationAnalyzer;

impl WasteAnalyzer for ContextAccumulationAnalyzer {
    fn name(&self) -> &str {
        "Context accumulation"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut seen_ids = HashSet::new();
        let mut input_sizes: Vec<u64> = vec![];
        let mut has_compact = false;

        for line in &session.lines {
            // Check for compact_boundary
            if line.line_type == "compact_boundary" {
                has_compact = true;
            }

            if line.line_type != "assistant" {
                continue;
            }
            let Some(ref msg) = line.message else {
                continue;
            };
            // Deduplicate by message id
            if let Some(ref id) = msg.id {
                if !seen_ids.insert(id.clone()) {
                    continue;
                }
            }
            let Some(ref usage) = msg.usage else {
                continue;
            };
            let input = usage.input_tokens.unwrap_or(0)
                + usage.cache_read_input_tokens.unwrap_or(0);
            if input > 0 {
                input_sizes.push(input);
            }
        }

        if input_sizes.len() < 2 || has_compact {
            return vec![];
        }

        let first = input_sizes[0];
        let last = *input_sizes.last().unwrap();

        // If input context grew by >50% from first to last turn
        if first == 0 || last <= first * 3 / 2 {
            return vec![];
        }

        let waste = last - first;

        vec![WasteFinding {
            category: "Context accumulation".to_string(),
            description: format!(
                "Input context grew from {} to {} tokens ({:.0}% increase) without compaction",
                first,
                last,
                ((last as f64 - first as f64) / first as f64) * 100.0
            ),
            estimated_tokens: waste,
            details: vec![
                format!("first turn input: {} tokens", first),
                format!("last turn input: {} tokens", last),
                format!("turns: {}", input_sizes.len()),
            ],
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

    fn assistant_line(id: &str, input_tokens: u64, cache_read: u64) -> JsonlLine {
        JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some(id.to_string()),
                role: Some("assistant".to_string()),
                content: None,
                usage: Some(Usage {
                    input_tokens: Some(input_tokens),
                    output_tokens: Some(100),
                    cache_read_input_tokens: Some(cache_read),
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

    #[test]
    fn test_detects_growth() {
        let session = make_session(vec![
            assistant_line("m1", 1000, 0),
            assistant_line("m2", 1500, 0),
            assistant_line("m3", 2000, 0),
        ]);
        let findings = ContextAccumulationAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "Context accumulation");
        assert_eq!(findings[0].estimated_tokens, 1000); // 2000 - 1000
    }

    #[test]
    fn test_no_flag_when_compact_present() {
        let mut lines = vec![
            assistant_line("m1", 1000, 0),
        ];
        lines.push(JsonlLine {
            line_type: "compact_boundary".to_string(),
            subtype: None,
            message: None,
            timestamp: None,
            uuid: None,
            message_id: None,
            snapshot: None,
            operation: None,
            content: None,
        });
        lines.push(assistant_line("m2", 2000, 0));
        let session = make_session(lines);
        let findings = ContextAccumulationAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_no_flag_when_growth_small() {
        let session = make_session(vec![
            assistant_line("m1", 1000, 0),
            assistant_line("m2", 1200, 0),
        ]);
        let findings = ContextAccumulationAnalyzer.analyze(&session);
        assert!(findings.is_empty());
    }
}

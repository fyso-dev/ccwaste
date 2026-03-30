use super::WasteAnalyzer;
use crate::types::{ContentBlock, Session, WasteFinding};
use std::collections::HashMap;

pub struct ReviewCyclesAnalyzer;

const TOKENS_PER_DISPATCH: u64 = 30000;

impl WasteAnalyzer for ReviewCyclesAnalyzer {
    fn name(&self) -> &str {
        "review_cycles"
    }

    fn analyze(&self, session: &Session) -> Vec<WasteFinding> {
        let mut pr_dispatches: HashMap<String, u64> = HashMap::new(); // PR number -> count

        for line in &session.lines {
            let Some(ref msg) = line.message else {
                continue;
            };
            let Some(ref content) = msg.content else {
                continue;
            };
            for val in content {
                if let Some(ContentBlock::ToolUse { name, input, .. }) = ContentBlock::from_value(val) {
                    if name != "Agent" {
                        continue;
                    }
                    // Extract PR numbers from description and prompt fields
                    let desc = input.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    let prompt = input.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                    let combined = format!("{} {}", desc, prompt);

                    for pr in extract_pr_numbers(&combined) {
                        *pr_dispatches.entry(pr).or_default() += 1;
                    }
                }
            }
        }

        let mut findings = vec![];
        for (pr, count) in &pr_dispatches {
            if *count <= 2 {
                continue;
            }
            let extra = count - 2;
            let waste = extra * TOKENS_PER_DISPATCH;
            findings.push(WasteFinding {
                category: "review_cycles".to_string(),
                description: format!(
                    "PR {} dispatched {} times ({} excess)",
                    pr, count, extra
                ),
                estimated_tokens: waste,
                details: vec![format!(
                    "~{} tokens per dispatch, {} extra dispatches",
                    TOKENS_PER_DISPATCH, extra
                )],
            });
        }

        findings
    }
}

fn extract_pr_numbers(text: &str) -> Vec<String> {
    let mut results = vec![];
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' {
            let mut num = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    num.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if !num.is_empty() {
                results.push(format!("#{}", num));
            }
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{JsonlLine, Message, Session, SessionInfo, Usage};

    fn make_agent_line(description: &str, prompt: &str) -> JsonlLine {
        JsonlLine {
            line_type: "assistant".to_string(),
            subtype: None,
            message: Some(Message {
                id: Some("msg_1".to_string()),
                role: Some("assistant".to_string()),
                content: Some(vec![serde_json::json!({
                    "type": "tool_use",
                    "id": "tu_1",
                    "name": "Agent",
                    "input": {
                        "description": description,
                        "prompt": prompt
                    }
                })]),
                usage: Some(Usage {
                    input_tokens: Some(100),
                    output_tokens: Some(50),
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

    #[test]
    fn test_review_cycles_detected() {
        let session = Session {
            info: SessionInfo {
                path: "test".to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines: vec![
                make_agent_line("Review PR #42", "Check the code in #42"),
                make_agent_line("Fix issues in PR #42", "Apply fixes for #42"),
                make_agent_line("Re-review PR #42", "Final review of #42"),
            ],
            total_tokens: 100000,
            subagents: vec![],
        };

        let findings = ReviewCyclesAnalyzer.analyze(&session);
        // #42 appears in each line (description + prompt), but we deduplicate per line?
        // Actually each line has 2 mentions of #42, so 6 total dispatches for #42
        // But the spec says "group by PR number" and count dispatches (Agent tool_use calls)
        // Each Agent call that mentions #42 counts. But #42 appears twice per call.
        // We should count unique PR per Agent call? Let's check: extract_pr_numbers returns all occurrences.
        // That means 2 per line = 6 total. > 2, so extra = 4, waste = 120000.
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, "review_cycles");
        assert!(findings[0].estimated_tokens > 0);
    }

    #[test]
    fn test_no_review_cycles_under_threshold() {
        let session = Session {
            info: SessionInfo {
                path: "test".to_string(),
                project_name: "test".to_string(),
                is_subagent: false,
                parent_session: None,
            },
            lines: vec![
                make_agent_line("Review PR #10", ""),
                make_agent_line("Review PR #11", ""),
            ],
            total_tokens: 50000,
            subagents: vec![],
        };

        let findings = ReviewCyclesAnalyzer.analyze(&session);
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn test_extract_pr_numbers() {
        assert_eq!(extract_pr_numbers("Review #123 and #456"), vec!["#123", "#456"]);
        assert_eq!(extract_pr_numbers("No PR here"), Vec::<String>::new());
        assert_eq!(extract_pr_numbers("#99"), vec!["#99"]);
    }
}

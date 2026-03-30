use crate::types::{ContentBlock, JsonlLine, Session, SessionInfo};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn parse_jsonl_file(path: &Path) -> Result<Vec<JsonlLine>, String> {
    let file =
        File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);
    let mut lines = vec![];
    for (i, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("Read error at line {}: {}", i, e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<JsonlLine>(trimmed) {
            Ok(parsed) => lines.push(parsed),
            Err(_) => continue,
        }
    }
    Ok(lines)
}

pub fn build_session(info: SessionInfo, lines: Vec<JsonlLine>) -> Session {
    let total_tokens = compute_total_tokens(&lines);
    Session {
        info,
        lines,
        total_tokens,
        subagents: vec![],
    }
}

fn compute_total_tokens(lines: &[JsonlLine]) -> u64 {
    let mut total: u64 = 0;
    let mut seen_message_ids = std::collections::HashSet::new();
    for line in lines {
        if line.line_type != "assistant" {
            continue;
        }
        let Some(ref msg) = line.message else {
            continue;
        };
        if let Some(ref id) = msg.id {
            if !seen_message_ids.insert(id.clone()) {
                continue;
            }
        }
        if let Some(ref usage) = msg.usage {
            total += usage.input_tokens.unwrap_or(0);
            total += usage.output_tokens.unwrap_or(0);
            total += usage.cache_creation_input_tokens.unwrap_or(0);
            total += usage.cache_read_input_tokens.unwrap_or(0);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_session_from_fixture() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/minimal.jsonl");
        let session = parse_jsonl_file(&path).unwrap();
        assert_eq!(session.len(), 2);
        assert_eq!(session[0].line_type, "system");
        assert_eq!(session[1].line_type, "assistant");
        let usage = session[1]
            .message
            .as_ref()
            .unwrap()
            .usage
            .as_ref()
            .unwrap();
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(20));
    }

    #[test]
    fn test_parse_tool_use_content() {
        let line = r#"{"type":"assistant","message":{"id":"msg_2","role":"assistant","content":[{"type":"tool_use","id":"tu_1","name":"Read","input":{"file_path":"/tmp/foo.rs"}}],"usage":{"input_tokens":50,"output_tokens":10}},"timestamp":"2026-03-30T10:00:00Z","uuid":"ghi"}"#;
        let parsed: crate::types::JsonlLine = serde_json::from_str(line).unwrap();
        let content = parsed.message.unwrap().content.unwrap();
        let block = ContentBlock::from_value(&content[0]).unwrap();
        match block {
            ContentBlock::ToolUse { name, input, .. } => {
                assert_eq!(name, "Read");
                assert_eq!(input["file_path"], "/tmp/foo.rs");
            }
            _ => panic!("Expected ToolUse"),
        }
    }
}

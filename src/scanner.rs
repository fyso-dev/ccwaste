use crate::types::SessionInfo;
use chrono::Local;
use chrono::NaiveDate;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FoundSession {
    pub main_jsonl: PathBuf,
    pub subagent_jsonls: Vec<PathBuf>,
    pub project_name: String,
}

pub fn find_sessions_for_today() -> Vec<FoundSession> {
    let today = Local::now().date_naive();
    let claude_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude/projects");
    if !claude_dir.exists() {
        return vec![];
    }
    find_sessions_in_dir(&claude_dir, today)
}

fn find_sessions_in_dir(base: &Path, date: NaiveDate) -> Vec<FoundSession> {
    let mut sessions = vec![];
    let Ok(projects) = fs::read_dir(base) else {
        return sessions;
    };
    for project_entry in projects.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }
        let project_name = extract_project_name(&project_path);
        let Ok(entries) = fs::read_dir(&project_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            if !modified_on_date(&path, date) {
                continue;
            }
            let session_id = path.file_stem().unwrap().to_string_lossy().to_string();
            let subagent_dir = project_path.join(&session_id).join("subagents");
            let subagent_jsonls = if subagent_dir.exists() {
                find_jsonl_files_in(&subagent_dir, date)
            } else {
                vec![]
            };
            sessions.push(FoundSession {
                main_jsonl: path,
                subagent_jsonls,
                project_name: project_name.clone(),
            });
        }
    }
    sessions
}

fn find_jsonl_files_in(dir: &Path, date: NaiveDate) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .flatten()
        .filter(|e| {
            let p = e.path();
            p.extension().and_then(|e| e.to_str()) == Some("jsonl") && modified_on_date(&p, date)
        })
        .map(|e| e.path())
        .collect()
}

fn modified_on_date(path: &Path, date: NaiveDate) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let modified_date: chrono::DateTime<Local> = modified.into();
    modified_date.date_naive() == date
}

fn extract_project_name(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let parts: Vec<&str> = name.split('-').collect();
    if parts.len() >= 2 {
        parts[parts.len() - 1].to_string()
    } else {
        name.to_string()
    }
}

pub fn find_jsonl_files(dir: &Path) -> Vec<SessionInfo> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
        .map(|e| SessionInfo {
            path: e.path().to_string_lossy().to_string(),
            project_name: "test".to_string(),
            is_subagent: false,
            parent_session: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_find_jsonl_files_in_dir() {
        let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let files = find_jsonl_files(&fixture_dir);
        assert!(!files.is_empty());
        assert!(files.iter().any(|f| f.path.ends_with("minimal.jsonl")));
    }
}

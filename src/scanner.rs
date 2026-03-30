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

pub fn find_sessions(days: u32, project_dir: Option<&str>, claude_base: Option<&std::path::Path>) -> Vec<FoundSession> {
    let today = Local::now().date_naive();
    let since = today - chrono::Duration::days(days as i64 - 1);
    let claude_dir = match claude_base {
        Some(base) => base.join("projects"),
        None => dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude/projects"),
    };
    if !claude_dir.exists() {
        return vec![];
    }

    // If project_dir given, find the matching project folder directly
    if let Some(dir) = project_dir {
        let encoded = dir.replace('/', "-");
        let project_path = claude_dir.join(&encoded);
        if project_path.exists() {
            return find_sessions_in_project(&project_path, since, today);
        }
        // Fallback: try partial match
        if let Ok(entries) = fs::read_dir(&claude_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains(&encoded) || name.ends_with(&encoded) {
                    return find_sessions_in_project(&entry.path(), since, today);
                }
            }
        }
        return vec![];
    }

    find_sessions_in_dir(&claude_dir, since, today)
}

fn find_sessions_in_project(project_path: &Path, since: NaiveDate, until: NaiveDate) -> Vec<FoundSession> {
    let mut sessions = vec![];
    let project_name = extract_project_name(project_path);
    let Ok(entries) = fs::read_dir(project_path) else { return sessions };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") { continue; }
        if !modified_in_range(&path, since, until) { continue; }
        let session_id = path.file_stem().unwrap().to_string_lossy().to_string();
        let subagent_dir = project_path.join(&session_id).join("subagents");
        let subagent_jsonls = if subagent_dir.exists() {
            find_jsonl_files_in(&subagent_dir, since, until)
        } else {
            vec![]
        };
        sessions.push(FoundSession {
            main_jsonl: path,
            subagent_jsonls,
            project_name: project_name.clone(),
        });
    }
    sessions
}

fn find_sessions_in_dir(base: &Path, since: NaiveDate, until: NaiveDate) -> Vec<FoundSession> {
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
            if !modified_in_range(&path, since, until) {
                continue;
            }
            let session_id = path.file_stem().unwrap().to_string_lossy().to_string();
            let subagent_dir = project_path.join(&session_id).join("subagents");
            let subagent_jsonls = if subagent_dir.exists() {
                find_jsonl_files_in(&subagent_dir, since, until)
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

fn find_jsonl_files_in(dir: &Path, since: NaiveDate, until: NaiveDate) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .flatten()
        .filter(|e| {
            let p = e.path();
            p.extension().and_then(|e| e.to_str()) == Some("jsonl")
                && modified_in_range(&p, since, until)
        })
        .map(|e| e.path())
        .collect()
}

fn modified_in_range(path: &Path, since: NaiveDate, until: NaiveDate) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let modified_date: chrono::DateTime<Local> = modified.into();
    let date = modified_date.date_naive();
    date >= since && date <= until
}

fn extract_project_name(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    // Format: -Users-user-Documents-work-fyso-usage
    // We want the last 2 meaningful segments: "fyso/usage"
    let parts: Vec<&str> = name.split('-').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 4 {
        // Skip Users-user-Documents-work prefix, take last 2
        let meaningful: Vec<&str> = parts.iter()
            .skip_while(|p| ["Users", "Documents", "work"].contains(p)
                || p.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) == false)
            .copied()
            .collect();
        if meaningful.len() >= 2 {
            format!("{}/{}", meaningful[meaningful.len() - 2], meaningful[meaningful.len() - 1])
        } else if meaningful.len() == 1 {
            meaningful[0].to_string()
        } else if parts.len() >= 2 {
            format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else {
            parts.last().unwrap_or(&"unknown").to_string()
        }
    } else if parts.len() >= 2 {
        format!("{}/{}", parts[parts.len() - 2], parts[parts.len() - 1])
    } else {
        parts.last().unwrap_or(&"unknown").to_string()
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

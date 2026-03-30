mod analyzers;
mod json_report;
mod parser;
mod report;
mod scanner;
mod types;

use chrono::Local;
use clap::Parser;
use types::{Report, SessionInfo, SessionReport};

#[derive(Parser)]
#[command(name = "ccwaste", version, about = "Claude Code conversation waste analyzer")]
struct Cli {
    /// Output report as JSON
    #[arg(long)]
    json: bool,
}

fn main() {
    let cli = Cli::parse();
    let today = Local::now().date_naive();

    let found = scanner::find_sessions_for_today();
    if found.is_empty() {
        if cli.json {
            let empty = Report {
                date: today.to_string(),
                sessions: vec![],
            };
            json_report::print_json(&empty);
        } else {
            eprintln!("No conversations found for today ({}).", today);
        }
        return;
    }

    let mut session_reports: Vec<SessionReport> = Vec::new();

    for fs in &found {
        let lines = match parser::parse_jsonl_file(&fs.main_jsonl) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Warning: {}", e);
                continue;
            }
        };

        let info = SessionInfo {
            path: fs.main_jsonl.to_string_lossy().to_string(),
            project_name: fs.project_name.clone(),
            is_subagent: false,
            parent_session: None,
        };

        let mut session = parser::build_session(info, lines);

        // Parse subagent jsonls
        for sub_path in &fs.subagent_jsonls {
            let sub_lines = match parser::parse_jsonl_file(sub_path) {
                Ok(l) => l,
                Err(_) => continue,
            };
            let sub_info = SessionInfo {
                path: sub_path.to_string_lossy().to_string(),
                project_name: fs.project_name.clone(),
                is_subagent: true,
                parent_session: Some(fs.main_jsonl.to_string_lossy().to_string()),
            };
            let sub_session = parser::build_session(sub_info, sub_lines);
            session.subagents.push(sub_session);
        }

        let findings = analyzers::run_all(&session);
        let wasted_tokens: u64 = findings.iter().map(|f| f.estimated_tokens).sum();
        let total = session.total_tokens
            + session
                .subagents
                .iter()
                .map(|s| s.total_tokens)
                .sum::<u64>();
        let waste_ratio = if total > 0 {
            wasted_tokens as f64 / total as f64
        } else {
            0.0
        };

        session_reports.push(SessionReport {
            project_name: fs.project_name.clone(),
            total_tokens: total,
            wasted_tokens,
            waste_ratio,
            findings,
            subagent_count: session.subagents.len(),
        });
    }

    // Sort by waste_ratio descending
    session_reports.sort_by(|a, b| b.waste_ratio.partial_cmp(&a.waste_ratio).unwrap_or(std::cmp::Ordering::Equal));

    let report = Report {
        date: today.to_string(),
        sessions: session_reports,
    };

    if cli.json {
        json_report::print_json(&report);
    } else {
        report::print_report(&report);
    }
}

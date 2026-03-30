use crate::types::Report;
use serde::Serialize;

#[derive(Serialize)]
struct JsonReport {
    date: String,
    summary: JsonSummary,
    categories: Vec<JsonCategory>,
    sessions: Vec<JsonSession>,
}

#[derive(Serialize)]
struct JsonSummary {
    sessions_scanned: usize,
    subagents_scanned: usize,
    total_tokens: u64,
    wasted_tokens: u64,
    waste_ratio: f64,
}

#[derive(Serialize)]
struct JsonCategory {
    name: String,
    tokens: u64,
    percentage: f64,
}

#[derive(Serialize)]
struct JsonSession {
    project_name: String,
    total_tokens: u64,
    wasted_tokens: u64,
    waste_ratio: f64,
    subagent_count: usize,
    findings: Vec<JsonFinding>,
}

#[derive(Serialize)]
struct JsonFinding {
    category: String,
    description: String,
    estimated_tokens: u64,
    details: Vec<String>,
}

pub fn print_json(report: &Report) {
    let total_wasted = report.total_wasted();
    let categories: Vec<JsonCategory> = report
        .category_totals()
        .into_iter()
        .map(|(name, tokens)| {
            let percentage = if total_wasted > 0 {
                tokens as f64 / total_wasted as f64
            } else {
                0.0
            };
            JsonCategory {
                name,
                tokens,
                percentage: (percentage * 100.0).round() / 100.0,
            }
        })
        .collect();

    let sessions: Vec<JsonSession> = report
        .sessions
        .iter()
        .map(|s| JsonSession {
            project_name: s.project_name.clone(),
            total_tokens: s.total_tokens,
            wasted_tokens: s.wasted_tokens,
            waste_ratio: (s.waste_ratio * 100.0).round() / 100.0,
            subagent_count: s.subagent_count,
            findings: s
                .findings
                .iter()
                .map(|f| JsonFinding {
                    category: f.category.clone(),
                    description: f.description.clone(),
                    estimated_tokens: f.estimated_tokens,
                    details: f.details.clone(),
                })
                .collect(),
        })
        .collect();

    let json_report = JsonReport {
        date: report.date.clone(),
        summary: JsonSummary {
            sessions_scanned: report.sessions.len(),
            subagents_scanned: report.sessions.iter().map(|s| s.subagent_count).sum(),
            total_tokens: report.total_tokens(),
            wasted_tokens: report.total_wasted(),
            waste_ratio: (report.waste_ratio() * 100.0).round() / 100.0,
        },
        categories,
        sessions,
    };

    match serde_json::to_string_pretty(&json_report) {
        Ok(json) => println!("{}", json),
        Err(e) => eprintln!("Failed to serialize report: {}", e),
    }
}

use crate::types::{Report, ProjectReport, SessionReport, WasteFinding};
use colored::Colorize;

pub fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        let val = n as f64 / 1_000_000.0;
        if val >= 10.0 {
            format!("{:.0}M", val)
        } else {
            format!("{:.1}M", val)
        }
    } else if n >= 1_000 {
        let val = n as f64 / 1_000.0;
        if val >= 100.0 {
            format!("{:.0}K", val)
        } else {
            format!("{:.1}K", val)
        }
    } else {
        format!("{}", n)
    }
}

fn make_bar(ratio: f64, width: usize) -> String {
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn color_by_ratio(name: &str, ratio: f64) -> colored::ColoredString {
    if ratio > 0.3 {
        name.red().bold()
    } else if ratio > 0.15 {
        name.yellow().bold()
    } else {
        name.green().bold()
    }
}

fn print_findings(findings: &[WasteFinding]) {
    let count = findings.len();
    for (i, finding) in findings.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└─" } else { "├─" };
        println!(
            "    {} {}: {}",
            connector,
            finding.category,
            format_tokens(finding.estimated_tokens)
        );
        let detail_prefix = if is_last { "       " } else { "    │  " };
        let total_details = finding.details.len();
        for detail in finding.details.iter().take(3) {
            println!("{}{}", detail_prefix, detail.dimmed());
        }
        if total_details > 3 {
            let more = format!("... and {} more", total_details - 3);
            println!("{}{}", detail_prefix, more.dimmed());
        }
    }
}

pub fn print_report(report: &Report) {
    let separator = "───────────────────────────────────────────";

    println!();
    println!("{}", format!("ccwaste — {}", report.date).bold());

    if report.show_sessions {
        print_sessions_view(report, separator);
    } else {
        print_project_view(report, separator);
    }

    // Summary always at the bottom
    print_summary(report, separator);
}

fn print_project_view(report: &Report, separator: &str) {
    let projects = report.grouped_by_project(&report.sort_order);

    let title = match report.sort_order.as_str() {
        "waste" => "Top Projects by Wasted Tokens",
        "tokens" => "Top Projects by Total Tokens",
        _ => "Top Projects by Waste Ratio",
    };
    println!();
    println!("{}", title.bold());
    println!("{}", separator.dimmed());

    // Show top 10 in ascending order so the worst is at the bottom (closest to summary)
    let top: Vec<&ProjectReport> = projects.iter().take(10).collect();
    for project in top.iter().rev() {
        let pct = format!("{}%", (project.waste_ratio * 100.0).round() as u64);
        let sessions_label = if project.session_count == 1 {
            "1 session".to_string()
        } else {
            format!("{} sessions", project.session_count)
        };
        let sub_label = if project.subagent_count > 0 {
            format!(", {} subagents", project.subagent_count)
        } else {
            String::new()
        };

        println!(
            "  {:<24} {:>6} tok  {:>6} waste ({})  {}",
            color_by_ratio(&project.project_name, project.waste_ratio),
            format_tokens(project.total_tokens),
            format_tokens(project.wasted_tokens),
            pct,
            format!("[{}{}]", sessions_label, sub_label).dimmed()
        );

        print_findings(&project.findings);
        println!();
    }

    if projects.len() > 10 {
        let remaining: u64 = projects.iter().skip(10).map(|p| p.wasted_tokens).sum();
        let remaining_count = projects.len() - 10;
        println!(
            "  {} ({} waste across {} projects)",
            "... and more".dimmed(),
            format_tokens(remaining),
            remaining_count
        );
        println!();
    }
}

fn print_sessions_view(report: &Report, separator: &str) {
    let title = match report.sort_order.as_str() {
        "waste" => "Top Sessions by Wasted Tokens",
        "tokens" => "Top Sessions by Total Tokens",
        _ => "Top Sessions by Waste Ratio",
    };
    println!();
    println!("{}", title.bold());
    println!("{}", separator.dimmed());

    let mut sorted: Vec<&SessionReport> = report.sessions.iter().collect();
    match report.sort_order.as_str() {
        "waste" => sorted.sort_by(|a, b| b.wasted_tokens.cmp(&a.wasted_tokens)),
        "tokens" => sorted.sort_by(|a, b| b.total_tokens.cmp(&a.total_tokens)),
        _ => sorted.sort_by(|a, b| b.waste_ratio.partial_cmp(&a.waste_ratio).unwrap_or(std::cmp::Ordering::Equal)),
    }

    let mut name_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for s in &sorted {
        *name_counts.entry(s.project_name.clone()).or_default() += 1;
    }
    let mut name_seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    let top: Vec<&&SessionReport> = sorted.iter().take(10).collect();
    for session in top.iter().rev() {
        let count = name_counts.get(&session.project_name).unwrap_or(&1);
        let display_name = if *count > 1 {
            let idx = name_seen.entry(session.project_name.clone()).or_default();
            *idx += 1;
            format!("{} #{}", session.project_name, idx)
        } else {
            session.project_name.clone()
        };

        let pct = format!("{}%", (session.waste_ratio * 100.0).round() as u64);
        let sub_info = if session.subagent_count > 0 {
            format!("  [{} subagents]", session.subagent_count)
        } else {
            String::new()
        };
        println!(
            "  {:<24} {:>6} tok  {:>6} waste ({}){}",
            color_by_ratio(&display_name, session.waste_ratio),
            format_tokens(session.total_tokens),
            format_tokens(session.wasted_tokens),
            pct,
            sub_info.dimmed()
        );

        print_findings(&session.findings);
        println!();
    }

    if sorted.len() > 10 {
        let remaining: u64 = sorted.iter().skip(10).map(|s| s.wasted_tokens).sum();
        let remaining_count = sorted.len() - 10;
        println!(
            "  {} ({} waste across {} sessions)",
            "... and more".dimmed(),
            format_tokens(remaining),
            remaining_count
        );
        println!();
    }
}

fn print_summary(report: &Report, separator: &str) {
    let total_tokens = report.total_tokens();
    let total_wasted = report.total_wasted();
    let waste_ratio = report.waste_ratio();

    println!("{}", "Summary".bold());
    println!("{}", separator.dimmed());

    println!(
        "  Sessions:     {}  (+ {} subagents)",
        report.session_count(),
        report.subagent_count()
    );
    println!("  Total tokens: ~{}", format_tokens(total_tokens));

    let waste_pct = format!("{}%", (waste_ratio * 100.0).round() as u64);
    println!(
        "  Wasted:       ~{} ({})",
        format_tokens(total_wasted),
        waste_pct.red()
    );

    // Category breakdown
    let categories = report.category_totals();
    if !categories.is_empty() {
        println!();
        let max_cat_tokens = categories.first().map(|(_, t)| *t).unwrap_or(1);
        for (name, tokens) in &categories {
            let ratio = if total_wasted > 0 {
                *tokens as f64 / total_wasted as f64
            } else {
                0.0
            };
            let bar_ratio = *tokens as f64 / max_cat_tokens as f64;
            let bar = make_bar(bar_ratio, 15);
            let pct = format!("{}%", (ratio * 100.0).round() as u64);
            println!(
                "  {:<24} {:>6}  {}  {}",
                name,
                format_tokens(*tokens),
                bar.yellow(),
                pct
            );
        }
    }

    // Recommendations based on top waste categories
    let top_cats: Vec<&str> = categories.iter()
        .filter(|(_, tokens)| *tokens > 0)
        .take(5)
        .map(|(name, _)| name.as_str())
        .collect();

    if !top_cats.is_empty() {
        println!();
        println!("{}", "Recommendations".bold());
        println!("{}", separator.dimmed());

        for cat in &top_cats {
            if let Some(rec) = recommendation(cat) {
                println!("  {} {}", "->".green(), rec);
            }
        }
    }

    println!();
}

fn recommendation(category: &str) -> Option<&'static str> {
    match category {
        "Review cycles" => Some(
            "Limit review/QA to 1 round per PR. Fix issues, then 1 re-review max."
        ),
        "Killed subagents" => Some(
            "Check MCP health before dispatching agents. Avoid killing and re-dispatching."
        ),
        "Context accumulation" => Some(
            "Use /compact or start new sessions for long tasks. Context grows every turn."
        ),
        "Metadata bloat" => Some(
            "Long sessions accumulate snapshots and hooks. Split work into shorter sessions."
        ),
        "File re-reads" => Some(
            "Include file content in subagent prompts instead of each one reading the same file."
        ),
        "Tool errors" => Some(
            "Validate paths and commands before running. Check git state before push."
        ),
        "Missing .claudeignore" => Some(
            "Add node_modules/, dist/, build/, .git/, target/ to .claudeignore"
        ),
        "Broad searches" => Some(
            "Always pass a specific path to Grep/Glob. Avoid searching from root."
        ),
        "Self-inflicted diffs" => Some(
            "Framework overhead — file snapshots after edits. Shorter sessions reduce impact."
        ),
        "Model overkill" => Some(
            "Use /model to switch to a cheaper model for simple file reads and commands."
        ),
        "Repeated ToolSearch" => Some(
            "Pre-register frequently used deferred tools to avoid repeated lookups."
        ),
        "CLAUDE.md bloat" => Some(
            "Trim CLAUDE.md — move detailed docs to separate files. It loads every session."
        ),
        _ => None,
    }
}

pub fn print_status(report: &Report) {
    let total_wasted = report.total_wasted();
    if total_wasted == 0 {
        println!("\u{2705}0");
        return;
    }

    let categories = report.category_totals();
    let top: Vec<String> = categories.iter()
        .take(2)
        .map(|(name, tokens)| {
            let pct = if total_wasted > 0 {
                ((*tokens as f64 / total_wasted as f64) * 100.0).round() as u64
            } else { 0 };
            format!("{} {} {}%", status_label(name), format_tokens(*tokens), pct)
        })
        .collect();

    let waste_pct = (report.waste_ratio() * 100.0).round() as u64;
    println!(
        "\u{1F5D1} {} ({}%) {}",
        format_tokens(total_wasted),
        waste_pct,
        top.join(" ")
    );
}

fn status_label(category: &str) -> &'static str {
    match category {
        "Review cycles" => "\u{1F504}reviews",          // 🔄reviews
        "Killed subagents" => "\u{1F480}subagents",     // 💀subagents
        "Context accumulation" => "\u{1F4C8}context",   // 📈context
        "Metadata bloat" => "\u{1F4E6}metadata",        // 📦metadata
        "File re-reads" => "\u{1F4D6}re-reads",         // 📖re-reads
        "Tool errors" => "\u{274C}errors",              // ❌errors
        "Missing .claudeignore" => "\u{1F6AB}ignore",   // 🚫ignore
        "Broad searches" => "\u{1F50D}search",          // 🔍search
        "Self-inflicted diffs" => "\u{1F4DD}diffs",     // 📝diffs
        "Model overkill" => "\u{1F4B0}model",           // 💰model
        "Repeated ToolSearch" => "\u{1F503}toolsearch",  // 🔃toolsearch
        "CLAUDE.md bloat" => "\u{1F4DC}claudemd",       // 📜claudemd
        _ => "\u{26A0}other",                           // ⚠other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(42), "42");
        assert_eq!(format_tokens(1_500), "1.5K");
        assert_eq!(format_tokens(12_800), "12.8K");
        assert_eq!(format_tokens(380_000), "380K");
        assert_eq!(format_tokens(1_200_000), "1.2M");
        assert_eq!(format_tokens(15_000_000), "15M");
    }
}

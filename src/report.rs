use crate::types::Report;
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

pub fn print_report(report: &Report) {
    let separator = "───────────────────────────────────";

    println!();
    println!("{}", format!("ccwaste — {}", report.date).bold());

    // Summary
    let total_tokens = report.total_tokens();
    let total_wasted = report.total_wasted();
    let waste_ratio = report.waste_ratio();
    let session_count = report.sessions.len();
    let subagent_count: usize = report.sessions.iter().map(|s| s.subagent_count).sum();

    let biggest_offender = report
        .sessions
        .iter()
        .max_by(|a, b| a.waste_ratio.partial_cmp(&b.waste_ratio).unwrap());

    println!();
    println!("{}", "Summary".bold());
    println!("{}", separator.dimmed());

    let sub_str = if subagent_count > 0 {
        format!(" (+ {} subagents)", subagent_count)
    } else {
        String::new()
    };
    println!(
        "  Sessions scanned:    {}{}",
        session_count, sub_str
    );
    println!(
        "  Total tokens:        ~{}",
        format_tokens(total_tokens)
    );

    let waste_pct = format!("{}%", (waste_ratio * 100.0).round() as u64);
    println!(
        "  Wasted tokens:       ~{} ({})",
        format_tokens(total_wasted),
        waste_pct.red()
    );

    if let Some(offender) = biggest_offender {
        let pct = format!("{}% waste", (offender.waste_ratio * 100.0).round() as u64);
        println!(
            "  Biggest offender:    {} ({})",
            offender.project_name.bold(),
            pct.red()
        );
    }

    // Top Waste Categories
    let categories = report.category_totals();
    if !categories.is_empty() {
        println!();
        println!("{}", "Top Waste Categories".bold());
        println!("{}", separator.dimmed());

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
                "  {:<22} {:>6} tokens  {}  {}",
                name,
                format_tokens(*tokens),
                bar.yellow(),
                pct
            );
        }
    }

    // Per Session
    println!();
    println!("{}", "Per Session".bold());
    println!("{}", separator.dimmed());

    for session in &report.sessions {
        let name_colored = if session.waste_ratio > 0.3 {
            session.project_name.red().bold()
        } else if session.waste_ratio > 0.15 {
            session.project_name.yellow().bold()
        } else {
            session.project_name.green().bold()
        };

        let pct = format!("{}%", (session.waste_ratio * 100.0).round() as u64);
        let sub_info = if session.subagent_count > 0 {
            format!("  [{} subagents]", session.subagent_count)
        } else {
            String::new()
        };
        println!(
            "  {}     {} tok  {} waste ({}){}",
            name_colored,
            format_tokens(session.total_tokens),
            format_tokens(session.wasted_tokens),
            pct,
            sub_info.dimmed()
        );

        let finding_count = session.findings.len();
        for (i, finding) in session.findings.iter().enumerate() {
            let connector = if i == finding_count - 1 {
                "└─"
            } else {
                "├─"
            };
            println!(
                "    {} {}: {}",
                connector,
                finding.category,
                format_tokens(finding.estimated_tokens)
            );
            for detail in &finding.details {
                let prefix = if i == finding_count - 1 {
                    "       "
                } else {
                    "│      "
                };
                println!("{}{}", prefix, detail.dimmed());
            }
        }
    }

    println!();
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

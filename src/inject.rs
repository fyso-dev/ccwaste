use crate::report::format_tokens;
use crate::types::Report;
use std::fs;
use std::path::PathBuf;

struct Rule {
    category: &'static str,
    rule: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        category: "Review cycles",
        rule: "NEVER review the same PR more than twice. One review + one re-review after fixes. If issues persist after re-review, escalate to the user instead of looping.",
    },
    Rule {
        category: "Killed subagents",
        rule: "Before dispatching agents that use MCP tools, verify the MCP server is healthy with a quick ping. If an agent is stuck on an MCP call for >30s, the server is likely down — do not kill and re-dispatch, ask the user to check the MCP server.",
    },
    Rule {
        category: "Context accumulation",
        rule: "For sessions longer than 20 turns, proactively suggest /compact to the user. Start a new session for unrelated tasks instead of continuing in a long one.",
    },
    Rule {
        category: "Metadata bloat",
        rule: "Prefer shorter, focused sessions over long-running ones. Each turn adds metadata overhead (file snapshots, hooks, queue ops) that accumulates.",
    },
    Rule {
        category: "File re-reads",
        rule: "When dispatching subagents that need the same file, include the file content in the prompt instead of having each subagent Read it independently. Cache file contents across related agent dispatches.",
    },
    Rule {
        category: "Tool errors",
        rule: "Before running git push, check git status first. Before editing a file, read it first. Before globbing a path, verify it exists. Avoid trial-and-error with tools — validate inputs.",
    },
    Rule {
        category: "Missing .claudeignore",
        rule: "If Grep/Glob results include node_modules/, dist/, build/, .git/, target/, .next/, __pycache__/, or .venv/, tell the user to add a .claudeignore with those patterns.",
    },
    Rule {
        category: "Broad searches",
        rule: "ALWAYS pass a specific path to Grep and Glob. Never search from the root directory or without a path. Narrow the search scope to the relevant directory.",
    },
    Rule {
        category: "Self-inflicted diffs",
        rule: "This is framework overhead from file-history-snapshots after edits. Shorter sessions reduce the cumulative impact.",
    },
    Rule {
        category: "Model overkill",
        rule: "For simple operations (reading files, listing directories, running basic commands), consider whether a cheaper model would suffice. Use /model to switch when doing bulk file reads.",
    },
    Rule {
        category: "Repeated ToolSearch",
        rule: "Remember tool schemas after the first ToolSearch. Do not search for the same tool (e.g. TaskCreate, TaskUpdate) multiple times in one session.",
    },
    Rule {
        category: "CLAUDE.md bloat",
        rule: "Keep CLAUDE.md concise. Move detailed documentation to separate files and use @includes. Every token in CLAUDE.md is loaded on every session start.",
    },
];

pub fn generate_rules(report: &Report) -> String {
    let categories = report.category_totals();
    let total_wasted = report.total_wasted();

    if total_wasted == 0 {
        return "# No waste detected — no rules needed.".to_string();
    }

    let mut lines = vec![];
    lines.push("# ccwaste rules — auto-generated token optimization rules".to_string());
    lines.push(format!(
        "# Based on {} of waste across {} sessions ({} days)",
        format_tokens(total_wasted),
        report.session_count(),
        report.date
    ));
    lines.push("# Re-run `ccwaste --inject` to update based on latest data".to_string());
    lines.push(String::new());

    let mut rule_count = 0;
    for (cat_name, tokens) in &categories {
        let pct = *tokens as f64 / total_wasted as f64;
        // Only include categories that represent >1% of waste
        if pct < 0.01 {
            continue;
        }

        if let Some(rule) = RULES.iter().find(|r| r.category == cat_name) {
            rule_count += 1;
            lines.push(format!(
                "## {} ({} — {:.0}% of waste)",
                rule.category,
                format_tokens(*tokens),
                pct * 100.0
            ));
            lines.push(String::new());
            lines.push(rule.rule.to_string());
            lines.push(String::new());
        }
    }

    if rule_count == 0 {
        lines.push("No significant waste categories detected.".to_string());
    }

    lines.join("\n")
}

pub fn inject_rules(report: &Report) {
    let rules_content = generate_rules(report);

    let claude_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude");

    let rules_path = claude_dir.join("ccwaste-rules.md");
    let claude_md_path = claude_dir.join("CLAUDE.md");

    // Write rules file
    match fs::write(&rules_path, &rules_content) {
        Ok(_) => eprintln!("Wrote {}", rules_path.display()),
        Err(e) => {
            eprintln!("Error writing {}: {}", rules_path.display(), e);
            return;
        }
    }

    // Add @include to CLAUDE.md if not already there
    let include_line = "@ccwaste-rules.md";
    let claude_md_content = fs::read_to_string(&claude_md_path).unwrap_or_default();

    if !claude_md_content.contains(include_line) {
        let new_content = if claude_md_content.is_empty() {
            format!("{}\n", include_line)
        } else {
            format!("{}\n\n{}\n", claude_md_content.trim_end(), include_line)
        };

        match fs::write(&claude_md_path, new_content) {
            Ok(_) => eprintln!("Added {} to {}", include_line, claude_md_path.display()),
            Err(e) => eprintln!("Error updating {}: {}", claude_md_path.display(), e),
        }
    } else {
        eprintln!("@include already in {}", claude_md_path.display());
    }

    eprintln!("\nRules will be active in all new Claude Code sessions.");
}

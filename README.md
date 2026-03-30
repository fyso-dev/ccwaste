# ccwaste

Claude Code conversation waste analyzer. Scans your JSONL session logs and identifies wasted tokens so you can fix the root causes.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/fyso-dev/ccwaste/main/install.sh | bash
```

Or build from source:

```bash
cargo install --git https://github.com/fyso-dev/ccwaste.git
```

## Usage

```bash
# Analyze last 30 days (default)
ccwaste

# Analyze last 7 days
ccwaste --days 7

# Show individual sessions
ccwaste --sessions

# Sort by worst waste ratio
ccwaste --order ratio

# Sort by total tokens consumed
ccwaste --order tokens

# Filter by project directory
ccwaste --project-dir /path/to/project

# JSON output
ccwaste --json

# Generate rules and inject into CLAUDE.md
ccwaste --inject

# Print rules to stdout
ccwaste --rules

# One-liner for statusLine
ccwaste --status
```

## Analyzers

| Analyzer | What it detects |
|---|---|
| :mag: Broad Searches | Grep/glob patterns that match too many files |
| :scroll: CLAUDE.md Bloat | Oversized CLAUDE.md files burning tokens on every message |
| :chart_with_upwards_trend: Context Accumulation | Sessions where context grows without bound |
| :open_file_folder: File Re-reads | Same file read multiple times in one session |
| :skull: Killed Subagents | Subagent tasks that were cancelled before completing |
| :package: Metadata Bloat | Excessive tool metadata in conversation turns |
| :eyes: Missing .claudeignore | Projects without .claudeignore letting noise into context |
| :hammer: Model Overkill | Opus used where Sonnet would suffice |
| :repeat: Repeated ToolSearch | Same tool searched for multiple times |
| :arrows_counterclockwise: Review Cycles | Fix-then-revert loops that waste tokens |
| :boom: Self-inflicted Diffs | Large diffs caused by the model's own edits |
| :x: Tool Errors | Tool calls that return errors |

## License

MIT

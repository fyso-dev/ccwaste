# ccwaste

CLI tool that analyzes Claude Code conversation logs to find wasted tokens. No AI — pure static analysis of JSONL files.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/fyso-dev/ccwaste/main/install.sh | bash
```

Or with Cargo:
```bash
cargo install --git https://github.com/fyso-dev/ccwaste --path ccwaste
```

## Usage

```bash
ccwaste                          # last 30 days, grouped by project
ccwaste -d 7                     # last week
ccwaste -d 1                     # today only
ccwaste -o ratio                 # sort by waste ratio
ccwaste --sessions               # per-session breakdown
ccwaste --project-dir /path/to   # filter by project
ccwaste --json                   # JSON output
ccwaste --status                 # one-liner for Claude Code statusLine
ccwaste --rules                  # print optimization rules
ccwaste --inject                 # write rules to ~/.claude/ccwaste-rules.md
```

## What it detects

| Analyzer | Detects |
|---|---|
| Review cycles | Same PR reviewed >2 times |
| Killed subagents | Agents interrupted mid-work |
| Context accumulation | Input tokens growing without /compact |
| Metadata bloat | file-history-snapshots, queue-ops, hooks |
| File re-reads | Same file read >2 times |
| Tool errors | Failed tool calls and retries |
| Missing .claudeignore | Results with node_modules, .git, etc |
| Broad searches | Grep/Glob without specific path |
| Self-inflicted diffs | Snapshots triggered by own edits |
| Model overkill | Opus used for simple Read/Glob/Bash |
| Repeated ToolSearch | Same tool schema queried multiple times |
| CLAUDE.md bloat | System prompt >10K tokens |

## StatusLine integration

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "ccwaste --status -d 7"
  }
}
```

Shows: `🗑 28M (1%) 💀subagents 10M 36% 🔄reviews 8.6M 30%`

## Prompt injection

Generate and inject optimization rules based on your actual waste data:

```bash
ccwaste --inject   # writes ~/.claude/ccwaste-rules.md + adds @include to CLAUDE.md
ccwaste --rules    # preview rules without writing
```

## License

MIT

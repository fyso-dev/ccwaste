# ccwasted

CLI tool that analyzes Claude Code conversation logs to find wasted tokens. No AI — pure static analysis of JSONL files.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/fyso-dev/ccwasted/main/install.sh | bash
```

Or with Cargo:
```bash
cargo install --git https://github.com/fyso-dev/ccwasted --path ccwasted
```

## Usage

```bash
ccwasted                          # last 30 days, grouped by project
ccwasted -d 7                     # last week
ccwasted -d 1                     # today only
ccwasted -o ratio                 # sort by waste ratio
ccwasted --sessions               # per-session breakdown
ccwasted --project-dir /path/to   # filter by project
ccwasted --json                   # JSON output
ccwasted --status                 # one-liner for Claude Code statusLine
ccwasted --rules                  # print optimization rules
ccwasted --inject                 # write rules to ~/.claude/ccwasted-rules.md
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

### Standalone

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "ccwasted --status -d 7"
  }
}
```

### Filter by current project

Use `--project-dir` to show waste only for the active project:

```json
{
  "statusLine": {
    "type": "command",
    "command": "ccwasted --status -d 7 --project-dir \"$PWD\""
  }
}
```

### Add to an existing statusLine script

If you already have a custom statusLine script (Node.js, bash, etc.), append ccwasted output:

```bash
# In your statusline script
WASTE=$(ccwasted --status -d 7 --project-dir "$PROJECT_DIR" 2>/dev/null)
echo "$YOUR_LINE | $WASTE"
```

For Node.js statusLine scripts that receive JSON on stdin:

```javascript
// Inside your statusline Node.js script
const { execSync } = require("child_process");
const projDir = d.workspace?.project_dir || "";
const dirFlag = projDir ? ` --project-dir ${JSON.stringify(projDir)}` : "";
try {
  const waste = execSync(`ccwasted --status -d 7${dirFlag} 2>/dev/null`, {
    timeout: 3000, encoding: "utf8"
  }).trim();
  if (waste) line += ` | ${waste}`;
} catch(e) {}
```

### Output format

```
🗑 28M (1%) 💀subagents 10M 36% 🔄reviews 8.6M 30%
│            │                   └─ 2nd top waste category
│            └─ top waste category with tokens and %
└─ total waste tokens (% of all tokens)
```

## Prompt injection

Generate and inject optimization rules based on your actual waste data:

```bash
ccwasted --inject   # writes ~/.claude/ccwasted-rules.md + adds @include to CLAUDE.md
ccwasted --rules    # preview rules without writing
```

## License

MIT

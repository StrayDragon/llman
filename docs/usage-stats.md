# Usage Stats (Codex / Claude Code / Cursor)

This repo adds a `stats` subcommand under each tool namespace to inspect *local* historical usage (tokens + time) scoped to the current working directory (v1).

## Commands

- `llman x codex stats`
- `llman x claude-code stats` (alias: `llman x cc stats`)
- `llman x cursor stats`

## Views

All tools support:

- `--view summary` (default): totals + coverage
- `--view trend`: day/week/month buckets (local timezone; week starts Monday)
- `--view sessions`: sorted by `end_ts` (desc)
- `--view session`: show one session (requires `--id` unless `--tui`)

## Common Flags

- `--group-by day|week|month` (trend only; default `day`)
- Time range (mutually exclusive):
  - `--last <Nd>` (days only, e.g. `7d`, `30d`, `90d`)
  - `--since <TIME>` / `--until <TIME>` (`RFC3339` or `YYYY-MM-DD` local)
- `--limit <N>` (sessions view only; default `200`; `0` = unlimited)
- `--format table|json` (default `table`)
- `--color auto|always|never` (table only; default `auto`)
  - `auto`: enable ANSI only when stdout is a TTY and `NO_COLOR` is not set
  - `always`: enable ANSI even when stdout is not a TTY
  - `never`: disable ANSI
  - JSON output is never colored
- `--tui` (open interactive TUI instead of printing output)
- `--verbose` (show full absolute paths in table/TUI)

## Tool-Specific Flags

Codex (`llman x codex stats`):

- `--state-db <PATH>`: override `~/.codex/state_*.sqlite` discovery
- `--with-breakdown`: parse rollout JSONL to populate input/output/cache/reasoning breakdown (slower; read-only)

Claude Code (`llman x claude-code stats` / `llman x cc stats`):

- `--projects-dir <PATH>`: override `~/.claude/projects` scan root
- `--no-sidechain`: exclude sidechain/subagent sessions (default: include; totals show primary vs sidechain vs overall)

Cursor (`llman x cursor stats`):

- `--db-path <PATH>`: override the workspace `state.vscdb`
- `--global-db-path <PATH>`: override the global `state.vscdb` (bubble KV source)

## Examples

Summary:

- `llman x codex stats`
- `llman x cc stats`
- `llman x cursor stats`

Color control:

- `llman x codex stats --color always` (force ANSI even when piped)
- `NO_COLOR=1 llman x codex stats` (disable ANSI in auto mode)

Trend:

- `llman x cc stats --view trend --last 30d --group-by week`
- `llman x codex stats --view trend --since 2026-02-01 --until 2026-03-01`

Sessions list:

- `llman x cursor stats --view sessions --last 7d --limit 20`

Session drilldown:

- `llman x codex stats --view session --id <THREAD_ID> --with-breakdown`
- `llman x cc stats --view session --id <SESSION_ID>`
- `llman x cursor stats --view session --id composer:<COMPOSER_ID>`

JSON output:

- `llman x codex stats --format json`

## Table Output

The default output (`--format table`) is intended for human readability (use JSON for scripts).

Example (summary):

```text
┌─────────────────┐
│ Tool    codex   │
│ Range   last 7d │
│ View    summary │
└─────────────────┘

┌──────────────────────────────┐
│ Metric                  Value │
╞══════════════════════════════╡
│ Sessions (total)            3 │
│ Sessions (known tokens)     3 │
│ Tokens (known-only total)  12 │
└──────────────────────────────┘
```

## Read-Only + Safety

- Reads only local on-disk state for each tool (SQLite / JSONL); no network requests.
- SQLite sources are opened in read-only mode.
- Output is usage metadata only (tokens/timestamps/ids/paths); the implementation avoids printing full conversation bodies.

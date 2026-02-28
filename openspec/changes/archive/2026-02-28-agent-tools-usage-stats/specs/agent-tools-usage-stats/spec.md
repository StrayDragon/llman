# agent-tools-usage-stats Specification

## Purpose
Provide local, per-project historical usage statistics (tokens + time) for Codex CLI, Claude Code, and Cursor, with a consistent UX (CLI table/JSON + optional TUI) and safe handling of missing token fields.

## ADDED Requirements

### Requirement: Stats subcommand exists for all three tools
The CLI MUST provide a `stats` subcommand under each tool namespace:
- `llman x codex stats`
- `llman x claude-code stats` (and the alias path `llman x cc stats`)
- `llman x cursor stats`

#### Scenario: Command help is discoverable
- **WHEN** the user runs `llman x codex stats --help`
- **THEN** the CLI prints help text for the stats command and exits successfully

### Requirement: Stats is scoped to current working directory (v1)
In the first version, stats MUST only include records whose recorded `cwd` equals the current process working directory (string/path exact match).

The stats command MUST NOT provide repo/all scope auto-detection or filtering options in v1.

#### Scenario: Default filters to current directory
- **WHEN** the user runs `llman x claude-code stats` in directory `/p/a`
- **THEN** the output excludes sessions whose recorded `cwd` is not exactly `/p/a`

### Requirement: Stats supports summary/trend/sessions/session views
The stats command MUST support `--view` with values:
- `summary` (default)
- `trend`
- `sessions`
- `session`

#### Scenario: Default view is summary
- **WHEN** the user runs `llman x cursor stats` without `--view`
- **THEN** the command renders the summary view

### Requirement: Sessions view provides stable session identifiers
The `sessions` view MUST include a stable session identifier `id` for every session record.

The `id` value MUST be accepted by the `session` view selector flag (see below) so users can reliably drill down.

For each tool, the `id` MUST be:
- Codex: the `threads.id` value from the Codex state database
- Claude Code: the `sessionId` value from Claude Code project logs
- Cursor (v1): `composer:<COMPOSER_ID>` for composer conversations

Note: Cursor "traditional chat" (tab-based) sessions are intentionally out of scope for v1 and may be added in v2 or later.

#### Scenario: Sessions view ids can be used to drill down
- **WHEN** the user runs `llman x codex stats --view sessions` and sees a row with id `X`
- **THEN** the user can run `llman x codex stats --view session --id X` to view that session’s details

### Requirement: Session detail selection uses `--id` in non-TUI mode
When `--view session` is used without `--tui`, the command MUST require `--id <ID>`.

If `--id` is missing, the command MUST return an error explaining that `--id` is required for the session view.

#### Scenario: Session view requires id
- **WHEN** the user runs `llman x claude-code stats --view session` without `--id`
- **THEN** the command returns an error that instructs the user to supply `--id`

### Requirement: Trend aggregation supports day/week/month
The stats command MUST support `--group-by` with values `day|week|month` (default `day`) for the `trend` view.

Buckets MUST be computed using the session end timestamp:
- Codex: thread `updated_at`
- Claude Code: the max timestamp of messages within the session (including sidechain if enabled)
- Cursor: the max `createdAt` of bubbles included in the session

Bucket boundaries MUST be computed in the machine's local time zone.

For `group-by=week`, the week MUST start on Monday (ISO-style week start).

#### Scenario: Group-by=month aggregates by calendar month
- **WHEN** the user runs `llman x codex stats --view trend --group-by month`
- **THEN** the output contains one bucket per calendar month present in the filtered dataset

### Requirement: Cursor sessions map to Cursor composer conversations (v1)
Cursor stats MUST treat each Cursor composer conversation as a session record.

For a Cursor session record:
- start timestamp MUST be the minimum `createdAt` among included bubbles
- end timestamp MUST be the maximum `createdAt` among included bubbles
- known token totals MUST be the sum of known bubble `tokenCount` values (unknown/missing tokenCount does not contribute)

Cursor bubble `createdAt` MUST be parsed as:
- RFC3339 string timestamp when it is a string
- epoch milliseconds when it is an integer
If `createdAt` cannot be parsed, that bubble MUST be ignored for timestamp calculations.

#### Scenario: Sessions view lists one record per conversation
- **WHEN** the user runs `llman x cursor stats --view sessions`
- **THEN** each row corresponds to exactly one Cursor composer conversation

#### Scenario: RFC3339 createdAt is accepted
- **WHEN** a Cursor bubble record has `createdAt = \"2026-02-28T10:00:00.000Z\"`
- **THEN** the session timestamps are computed using that parsed time

### Requirement: Output formats include table and JSON, plus optional TUI
The stats command MUST support:
- `--format table` (default)
- `--format json`
- `--tui` to open an interactive TUI (ratatui)

When `--tui` is set, the command MUST render the interactive UI instead of printing a table/JSON report.

#### Scenario: JSON output is machine-readable
- **WHEN** the user runs `llman x codex stats --format json`
- **THEN** stdout is valid JSON and contains the selected view result

### Requirement: Time range filtering is supported (all history by default)
The stats command MUST support optional time range filtering using one of:
- explicit bounds: `--since <TIME>` and/or `--until <TIME>`
- relative range: `--last <Nd>`

If neither `--since` nor `--until` nor `--last` is provided, the command MUST include all available history (no time filtering).

If `--last` is provided, the command MUST treat it as a relative window ending “now” and MUST NOT allow `--last` to be combined with `--since` or `--until` (the command MUST return an error).

`<TIME>` MUST accept:
- RFC3339 (e.g. `2026-02-01T00:00:00Z`)
- date-only `YYYY-MM-DD` (interpreted in the machine's local time zone)

For filtering, a session MUST be included when:
- `end_ts >= since` (if `since` is set)
- `end_ts < until` (if `until` is set)

For date-only input:
- `--since YYYY-MM-DD` MUST be interpreted as local `YYYY-MM-DD 00:00:00`
- `--until YYYY-MM-DD` MUST be interpreted as local start of the next day (exclusive), i.e. `YYYY-MM-DD + 1 day at 00:00:00`

For `--last <Nd>`:
- `N` MUST be a positive integer
- the unit MUST be `d` (days)
- `since = now_local - N days` and `until = now_local`

#### Scenario: Since filters old sessions out
- **WHEN** the user runs `llman x codex stats --since 2026-02-01T00:00:00Z`
- **THEN** sessions ending before that timestamp are excluded from all views

#### Scenario: Date-only until includes the whole day
- **WHEN** the user runs `llman x cursor stats --until 2026-02-01`
- **THEN** sessions ending on 2026-02-01 local time are included
- **AND** sessions ending on or after 2026-02-02 local time are excluded

#### Scenario: Last is mutually exclusive with since/until
- **WHEN** the user runs `llman x claude-code stats --last 7d --since 2026-02-01`
- **THEN** the command returns an error indicating incompatible flags

### Requirement: Sessions view is sorted and supports limiting
The `sessions` view MUST sort session records by end timestamp descending (most recent first).

The stats command MUST support `--limit <N>` to limit the number of sessions returned by the `sessions` view.

`--limit` MUST default to `200`.

If `--limit 0` is provided, the command MUST return all sessions (no limit).

`--limit` MUST NOT affect the `summary` or `trend` views (they MUST use all sessions after filtering).

#### Scenario: Limit applies only to sessions view
- **WHEN** the user runs `llman x cursor stats --view sessions --limit 10`
- **THEN** the sessions view returns 10 sessions
- **AND** `llman x cursor stats --view trend` still aggregates across all sessions in the filtered dataset

### Requirement: Data source paths can be overridden for testing and debugging
Each tool stats command MUST provide flags to override where it reads local state from:

- Codex stats MUST support `--state-db <PATH>` to override the default `~/.codex/state_*.sqlite` discovery.
- Claude Code stats MUST support `--projects-dir <PATH>` to override the default `~/.claude/projects` scan root.
- Cursor stats MUST support `--db-path <PATH>` to specify a workspace `state.vscdb` path, and MUST support `--global-db-path <PATH>` to override the default global `state.vscdb` path discovery when needed.

#### Scenario: Override path avoids reading real user state
- **WHEN** the user runs stats with an override path pointing at a temporary fixture
- **THEN** the command reads from the fixture paths instead of the default home directories

### Requirement: Path display is abbreviated by default and verbose is opt-in
In non-JSON output (table/TUI), the command MUST abbreviate displayed `cwd` paths by default:
- If the current directory is inside a git repo, the command SHOULD display repo-relative paths when the session cwd is within that repo.
- Otherwise, the command MUST display only the last two path segments.

The command MUST provide `--verbose` to show full absolute paths.

#### Scenario: Default hides full absolute path
- **WHEN** the user runs `llman x cursor stats` without `--verbose`
- **THEN** the sessions list does not print full absolute `cwd` paths

### Requirement: TUI provides an interactive filter form
When `--tui` is enabled, the UI MUST provide a way to open a “filter form” to edit query settings and re-run the scan, including:
- time range (`since` / `until` / all)
- group-by (`day|week|month`)
- tool-specific toggles (at minimum: Codex breakdown, Claude include-sidechain)

The TUI filter form MUST provide time range presets: `All`, `7d`, `30d`, `90d`.

The TUI MUST be stateless across runs: it MUST NOT persist the user's last selections to disk.

#### Scenario: TUI filter updates the view
- **WHEN** the user changes the time range in the TUI filter form and submits it
- **THEN** the displayed sessions/trend update to reflect the new range

### Requirement: Missing token fields are treated as unknown (no estimation)
If a record lacks token fields required for a metric, the command MUST treat that metric as unknown for that record.

The command MUST NOT estimate or infer token values from message text.

For any aggregated totals, the command MUST sum only known token values and MUST NOT treat unknown values as zero without making the distinction visible (either by leaving fields empty or by reporting “known-only totals”).

#### Scenario: Unknown token does not break aggregation
- **WHEN** some sessions in the filtered dataset have no token information
- **THEN** the command still renders the requested view successfully

### Requirement: Data sources are local and read-only
The stats implementation MUST read from the local on-disk state of each tool and MUST NOT perform network requests.

The implementation MUST NOT modify or write into the tool state directories (Codex/Claude/Cursor). For SQLite sources, the implementation MUST open databases in read-only mode.

Any additional files created for tests MUST use temporary directories.

#### Scenario: Offline execution
- **WHEN** the machine has no network connectivity
- **THEN** `llman x codex stats` still runs using local state

### Requirement: Claude Code sidechain is included by default and counted separately
Claude Code stats MUST include sidechain/subagent sessions by default.

Sidechain sessions MUST appear as separate session records in the `sessions` view (not silently merged into the primary session).

The summary/trend views MUST report both:
- primary-session known token totals
- sidechain-session known token totals
- overall known token totals (primary + sidechain)

The command MUST provide a flag to disable sidechain inclusion for users who want only the primary session records.

#### Scenario: Sidechain is visible and contributes to totals
- **WHEN** a Claude Code project has a primary session and an associated sidechain with token usage
- **THEN** the sessions view lists both sessions
- **AND** the default totals include both sessions’ known token usage while also showing separate primary vs sidechain totals

### Requirement: Known-token coverage is reported
Because some sessions may have unknown/missing token fields, the `summary` view MUST report at least:
- `total_sessions` (after filtering)
- `known_token_sessions` (sessions that have a known total token value)

The `trend` view MUST report the same coverage per bucket.

#### Scenario: Coverage is visible
- **WHEN** the dataset includes sessions with missing token info
- **THEN** the report includes coverage fields indicating known-token coverage

### Requirement: Codex breakdown is optional and off by default
Codex stats MUST support an option (for example `--with-breakdown`) that enables reading Codex rollout JSONL data to compute a token breakdown (input/output/cache/reasoning) when available.

By default, Codex stats MUST compute total tokens using the thread-level `tokens_used` summary only.

#### Scenario: Default uses thread tokens_used only
- **WHEN** the user runs `llman x codex stats` without the breakdown option
- **THEN** the command does not require parsing rollout JSONL files to produce totals

### Requirement: Tool-specific flags are stable
Codex stats MUST support a boolean flag `--with-breakdown` to enable parsing rollout JSONL for token breakdown.

Claude Code stats MUST support a boolean flag `--no-sidechain` to disable sidechain/subagent inclusion.

#### Scenario: no-sidechain disables sidechain
- **WHEN** the user runs `llman x claude-code stats --no-sidechain`
- **THEN** sidechain sessions are excluded from all views

### Requirement: Long-running scans provide progress feedback
When the command performs a long-running scan (especially when Codex breakdown is enabled), it MUST provide progress feedback to the user.

At minimum, when running in `--tui` mode, the UI MUST display a progress indicator that updates as work completes.

#### Scenario: Codex breakdown shows progress in TUI
- **WHEN** the user runs `llman x codex stats --tui --with-breakdown` and multiple rollout files are parsed
- **THEN** the TUI shows an updating progress indicator until the scan completes

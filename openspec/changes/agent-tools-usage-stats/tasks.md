## 0. Delivery phases (guide)

Phase 1 (MVP core + Codex CLI): 1.x + 2.x + 3.x
Phase 2 (Claude Code CLI): 4.x
Phase 3 (Cursor CLI): 5.x
Phase 4 (TUI + polish + docs): 6.x + 7.x

原则：每个复选框任务都应可在 1 个 PR 内完成，并附带可运行的验证（单测/集测/手动命令）。

## 1. Shared core (query/model/aggregation/render) — Phase 1

- [x] 1.1 Create `src/usage_stats/` core module skeleton and public interfaces (compiles; no tool wiring yet)
- [x] 1.2 Define core model types (`TokenUsage`, `SessionRecord`, `SessionId`, coverage fields) with serde support for JSON output
- [x] 1.3 Implement time range parsing: `--since/--until` (RFC3339 + `YYYY-MM-DD` local) and `--last <Nd>` (days only; mutually exclusive) + unit tests
- [x] 1.4 Implement v1 filtering: cwd exact-match + end_ts time filtering; ensure unknown token fields remain unknown (not coerced)
- [x] 1.5 Implement bucketing for `day|week|month` in local timezone, with Monday week start + unit tests for boundaries
- [x] 1.6 Implement view builders: `summary` (with coverage), `trend` (with per-bucket coverage), `sessions` (sorted), `session` (requires id in non-TUI)
- [x] 1.7 Implement renderers: `--format table|json`; JSON output must be stable and machine-readable + tests that parse JSON
- [x] 1.8 Implement path display shortening for table/TUI (repo-relative when possible else last-2-segments) + `--verbose` full paths + unit tests

## 2. CLI wiring (commands/args) — Phase 1

- [x] 2.1 Add `stats` subcommand to `x codex`, `x claude-code` (`x cc`), `x cursor` with shared args: `--view`, `--group-by`, `--since/--until/--last`, `--format`, `--tui`, `--verbose`
- [x] 2.2 Implement view-specific CLI args: `--id` (required for `--view session` when not `--tui`), `--limit` for `--view sessions` (default 200; `0` = unlimited)
- [x] 2.3 Add tool-specific flags + path overrides:
  - Codex: `--state-db <PATH>`, `--with-breakdown`
  - Claude: `--projects-dir <PATH>`, `--no-sidechain`
  - Cursor: `--db-path <PATH>`, `--global-db-path <PATH>`
- [x] 2.4 Add CLI-level integration tests for `--format json` (TempDir fixtures + `TestProcess` env isolation; never read real home state)

## 3. Codex source — Phase 1

- [x] 3.1 Implement Codex state DB discovery (`~/.codex/state_*.sqlite` choose highest) and `--state-db` override; open SQLite in read-only mode
- [x] 3.2 Map Codex `threads` rows to `SessionRecord` (stable id = `threads.id`; end_ts = `updated_at`; token total = `tokens_used`)
- [x] 3.3 Implement `--with-breakdown`: parse `rollout_path` JSONL to populate input/output/cache/reasoning when available; ignore malformed lines safely; never write back
- [x] 3.4 Add unit tests with minimal sqlite + rollout jsonl fixtures (cover: missing rollout_path, missing rate_limits, partial breakdown)
- [x] 3.5 Add Codex CLI JSON integration tests using fixture overrides (`--state-db`, optional breakdown)

## 4. Claude Code source — Phase 2

- [ ] 4.1 Implement Claude session discovery by scanning `--projects-dir` (default `~/.claude/projects`) for session JSONL; stable id = `sessionId`
- [ ] 4.2 Aggregate per-session token usage from `message.usage.*`; treat missing usage as unknown; compute end_ts as max message timestamp
- [ ] 4.3 Include sidechain sessions by default and surface them as separate `SessionRecord`s; implement `--no-sidechain` to exclude them
- [ ] 4.4 Ensure summary/trend show primary vs sidechain vs overall known-only totals + coverage fields
- [ ] 4.5 Add unit + CLI JSON integration tests using jsonl fixtures (primary + sidechain + missing usage + time filters)

## 5. Cursor source — Phase 3

- [ ] 5.1 Extend Cursor read layer to fetch bubble JSON from `cursorDiskKV` and parse `tokenCount` + `createdAt` (string RFC3339 or epoch ms); ignore unparseable bubbles for timestamps
- [ ] 5.2 Define Cursor session mapping (v1): one session per composer conversation; stable id `composer:<id>`; compute start/end from bubble createdAt
- [ ] 5.3 Implement `--db-path/--global-db-path` overrides without touching real user state; ensure all SQLite connections are read-only
- [ ] 5.4 Add unit tests with minimal vscdb fixtures (include both createdAt formats; missing tokenCount; empty conversations)
- [ ] 5.5 Add Cursor CLI JSON integration tests using fixture overrides

## 6. TUI (ratatui) — Phase 4

- [x] 6.1 Build shared stats TUI with tabs (Overview/Trend/Sessions/Session Detail) and a filter-form modal (All/7d/30d/90d presets; stateless across runs)
- [x] 6.2 Wire TUI to core view builders; implement “submit form → rerun query” flow; ensure `session` drilldown works without needing CLI `--id`
- [x] 6.3 Add progress reporting hooks for long scans; in particular show breakdown parsing progress for Codex `--with-breakdown`
- [x] 6.4 Add minimal TUI state tests (pure state transitions; no terminal snapshot dependencies)

## 7. Docs, safety, and robustness — Phase 4

- [ ] 7.1 Harden error handling (missing dirs/files, permission errors, malformed jsonl/sqlite records) with user-facing errors; never panic on bad data
- [ ] 7.2 Ensure outputs never print secrets; keep all operations read-only; document read-only guarantees
- [ ] 7.3 Document the new `stats` commands (examples for summary/trend/sessions/session; `--last`; `--with-breakdown`; path overrides; JSON output)

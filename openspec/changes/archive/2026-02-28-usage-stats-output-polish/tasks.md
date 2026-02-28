## 1. CLI table polish (comfy-table/tabled + color=auto)

- [x] 1.1 Add a shared `--color auto|always|never` flag to `StatsCliArgs` (default `auto`) and plumb it through to the table renderer (JSON unaffected)
- [x] 1.2 Implement color policy resolution: TTY detection + `NO_COLOR` handling + `--color` override; add focused unit tests (no snapshot)
- [x] 1.3 Replace `render_stats_table` implementation with a real table renderer (`comfy-table` preferred); ensure output contains no literal `\\t` and sanitizes cell content (`\\t`/newlines → spaces)
- [x] 1.4 Improve numeric formatting in table output (thousands separators, consistent unknown marker) and keep column sets consistent per view (Claude sessions includes sidechain column)
- [x] 1.5 Add integration tests that assert: (a) `--color auto` produces no ANSI in captured output, (b) `NO_COLOR=1` disables ANSI, (c) `--color always` includes ANSI (minimal `\\x1b[` match)
- [x] 1.6 Update `docs/usage-stats.md` with the new table output examples and `--color` behavior (auto/NO_COLOR)

## 2. TUI layout + visuals

- [x] 2.1 Refactor Sessions tab to use a `Table` widget with aligned columns and selection highlight; show Claude sidechain marker column; keep drilldown behavior
- [x] 2.2 Refactor Overview tab into metric “cards” (tokens/sessions/coverage/latest) and render coverage as a Gauge/Bar
- [x] 2.3 Refactor Trend tab into a chart + bucket detail view (BarChart/Sparkline + table); for Claude, support switching overall vs primary vs sidechain
- [x] 2.4 Standardize TUI styling (colors, highlight, help line) to match existing repo patterns (e.g. skills picker); ensure `NO_COLOR` only affects CLI table (TUI remains interactive)
- [x] 2.5 Add/adjust minimal TUI state tests for new components (pure state transitions; no terminal snapshot)

## 3. Verification

- [x] 3.1 Run `just check` and update any impacted tests; do a quick manual smoke pass for `llman x codex|cc|cursor stats` (table/json/tui)

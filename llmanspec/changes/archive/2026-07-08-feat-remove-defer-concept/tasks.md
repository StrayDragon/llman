# Tasks

- [x] T1: Simplify `TaskStatus` enum in `src/sdd/shared/tasks.rs` — remove Deferred/LegacyDefer/Cancelled variants, remove 3 Regex statics, simplify `classify_unchecked()`, remove report fields (deferred/legacy_defer/cancelled), update `completion_ratio()`, remove matching branches
- [x] T2: Delete `src/sdd/shared/orphans.rs` and remove from `mod.rs`
- [x] T3: Update `src/sdd/spec/validation.rs` — simplify `check_tasks_completion()` to only handle Completed/Pending, remove archived_change_ids/has_frozen params, update call sites, remove defer-related tests
- [x] T4: Update `src/sdd/shared/graph.rs` — remove `DeferEdge` struct, `collect_defer_edges()`, defer edge rendering in `render_nodes_and_edges()` and `render_mermaid()`, update tests
- [x] T5: Simplify `src/sdd/change/archive.rs` — remove Cancelled/Deferred special cases in gate logic, update tests
- [x] T6: Remove `Orphans` command variant from `src/sdd/command.rs`, remove imports and dispatch
- [x] T7: Update SKILL.md files — remove orphan/defer/`strict_defer` references from `llman-sdd-apply` and `llman-sdd-archive` (no changes needed)
- [x] T8: Run `just check` and fix all clippy/fmt issues
- [x] T9: Run `just test` — pass all tests, update/fix test expectations where needed
- [x] T10: Validate change: `just run sdd validate feat-remove-defer-concept --strict --no-interactive`

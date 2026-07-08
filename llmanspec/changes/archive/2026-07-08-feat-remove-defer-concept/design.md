# Design: Remove defer concept

## TaskStatus simplification

Before:
```rust
enum TaskStatus {
    Completed,
    Pending,
    Deferred { target: String },    // (defer → X)
    LegacyDefer { reason: String },  // (defer - reason)
    Cancelled { reason: String },    // (cancelled - reason)
}
```

After:
```rust
enum TaskStatus {
    Completed,
    Pending,
}
```

### Regex removal

Three static Regex values are removed: `RE_DEFER_LINKED`, `RE_CANCELLED`, `RE_DEFER_LEGACY`.

`classify_unchecked()` always returns `TaskStatus::Pending`.

### TasksReport simplification

Remove fields: `deferred`, `legacy_defer`, `cancelled`.

`completion_ratio()` becomes `completed as f64 / total as f64` — no excluded categories.

## Archive gate simplification

Before:
```rust
if report.pending > 0 { ... }   // only Pending + LegacyDefer counted
// min_completion_ratio also needed special case handling
```

After:
```rust
if report.pending > 0 { ... }   // all unchecked are pending now
// completion_ratio = completed/total, straightforward
```

## Validation simplification

`check_tasks_completion()`:
- Remove `TaskStatus::Deferred { target }` branch (defer target existence check)
- Remove `TaskStatus::LegacyDefer` branch
- Remove `TaskStatus::Cancelled` branch
- Keep only `TaskStatus::Pending` (with `strict_defer` gating) and `TaskStatus::Completed` (no issue)
- Remove `archived_change_ids` and `has_frozen` params (only used by defer checks)

## Modules to delete

- `src/sdd/shared/orphans.rs` (entire file, ~150 lines)

## Graph simplification

- Remove `DeferEdge` struct and `collect_defer_edges()` from `graph.rs`
- Remove defer edge rendering from `render_nodes_and_edges()`
- Remove `has_defer` flag from `render_mermaid()`

## Config simplification

Export `ArchiveConfig::strict_defer` (used by validation), remove nothing else from config.

## CLI simplification

- Remove `Orphans` variant from `SddCommands`
- Remove `orphans` import and command dispatch
- Remove `orphans.rs` from `mod.rs`

## SKILL.md updates

- `llman-sdd-apply/SKILL.md`: add stage guard instruction referencing `show --json`
- `llman-sdd-archive/SKILL.md`: remove orphan/`strict_defer` references

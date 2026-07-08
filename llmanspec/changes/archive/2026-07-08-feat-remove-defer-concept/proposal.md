---
depends_on: []
---

## Why

The `(defer → target)` task annotation creates a "fire-and-forget" mechanism that silently
accumulates invisible obligations. When change X archives with deferred tasks pointing to Y,
the agent implementing Y never sees them. Freeze makes this worse—the original tasks.md
becomes physically unscannable inside a 7z blob.

`(cancelled - reason)` and `(defer - reason)` annotate tasks that should not be executed,
bloating tasks.md with meta-commentary. If a task is wrong, it should be removed from the
execution checklist. Design rationale belongs in design.md or git history, not in the task list.

These three special statuses (`Deferred`, `LegacyDefer`, `Cancelled`) account for ~430 lines
of code and add conceptual complexity with zero agent-facing value.

## What Changes

1. Remove `TaskStatus::Deferred`, `TaskStatus::LegacyDefer`, `TaskStatus::Cancelled`.
   `TaskStatus` becomes a 2-variant enum: `Completed | Pending`.

2. Delete `src/sdd/shared/orphans.rs` entirely (was only detecting deferred-task targets).

3. Remove `collect_defer_edges()` and defer edge rendering from `src/sdd/shared/graph.rs`.

4. Remove `ArchiveConfig::strict_defer` from `src/sdd/project/config.rs`.

5. Simplify `validation.rs`: remove defer target existence checks, legacy defer warnings,
   cancelled skip logic.

6. Simplify `archive.rs`: remove `Cancelled`/`Deferred`-aware gate logic.

7. Update `llman-sdd-archive` SKILL.md to remove orphan-related commands and
   `strict_defer` references.

## Capabilities

- `sdd-workflow` — modifies task parsing, validation, archive gate, and orphans behavior

## Impact

- **Breaking**: old tasks.md files using `(defer → X)`, `(defer - reason)`, or
  `(cancelled - reason)` annotations will be parsed without special treatment (those
  items become regular `Pending`). Users should clean up their tasks.md before
  upgrading: remove deferred/cancelled items, or create followup changes.
- ~430 lines of code deleted, 0 new lines.
- Orphan detection subcommand removed (was only useful for defer).
- Graph output no longer shows defer dashed edges.
- Archive gate becomes simpler: only `report.pending > 0` blocks archiving.

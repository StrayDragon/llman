## Why

The `future.md` concept (deferred items / branch options tracked per-change) was misleading in practice: deferred items in `future.md` were routinely forgotten, never revisited, and gave a false sense of planning. The concept was a bad design.

The `future-planning` unit template injected guidance into multiple SDD skills, telling agents to classify deferred items as `now|later|drop` and map them to executable actions. This created noise without value.

## What Changes

- Remove `templates/sdd/{en,zh-Hans}/units/skills/future-planning.md` unit files
- Remove `{{ unit("skills/future-planning") }}` from all skill templates (10 files across en/zh-Hans)
- Remove `"skills/future-planning.md"` from `UNIT_FILES` in `src/sdd/project/templates.rs` and the two `include_str!` blocks
- Remove stale test assertions referencing "Future-to-Execution Planning" and "llmanspec/changes/<id>/future.md" from `tests/sdd_integration_tests.rs`
- Remove entire `test_sdd_validate_change_without_future_md_still_succeeds` test (no longer relevant)
- Delete `llmanspec/specs/sdd-future-changes/` spec entirely (was the contract for future.md)
- Remove r23 from `llmanspec/specs/sdd-workflow/spec.toon` (mandated future-guidance in skills)
- Merge content from existing `future.md` files into respective `design.md` files; delete the `future.md` files
- Add "check spec valid_scope integrity" step to propose/apply skill preflight (scope integrity hook)

## Capabilities

- `sdd-templates` (template units, skill templates)
- `sdd-workflow` (workflow spec updated)

## Impact

- Removes misleading future-planning guidance from all SDD skills
- No behavioral contract changes to end-user CLI commands
- Adds a scope integrity check in propose/apply preflight to automatically detect orphaned valid_scope references

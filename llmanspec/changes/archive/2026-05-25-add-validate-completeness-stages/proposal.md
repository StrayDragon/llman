---
depends_on: []
blocks: []
---

# Add Validate Completeness Stages

## Why

Currently `llman sdd validate` treats all intermediate change states silently — a draft proposal with only `proposal.md` passes validation with zero feedback, and having `tasks.md` without `design.md` (violating the intended workflow order) also passes without warning. This makes it hard for users to understand their progress or catch workflow ordering mistakes.

## What Changes

1. **Stage detection**: Introduce a `completeness_check` step in validation that scans the change directory and classifies the change into one of four stages: `draft`, `specified`, `designed`, `full`.
2. **Design → Tasks constraint**: Enforce that `tasks.md` MUST NOT exist without `design.md`. Violating this constraint produces an ERROR.
3. **Graded messaging**: In non-strict mode, emit `[INFO]` for missing-but-optional artifacts; in strict mode, require at least `designed` stage (missing `tasks.md` gives `[WARN]`, missing `design.md` with existing `tasks.md` gives `[ERROR]`).
4. **List stage column**: Add a `stage` column to `llman sdd list` output (both text and JSON) to show each change's completeness stage at a glance.

## Capabilities

- `sdd-workflow`

## Impact

- **Validate logic** (`src/sdd/spec/validation.rs`, `src/sdd/shared/validate.rs`): New completeness check function and integration into `validate_change_full`.
- **List output** (`src/sdd/shared/list.rs`): New stage column in text output; new `stage` field in JSON output.
- **i18n** (`locales/`): New translation keys for stage messages.
- **Tests** (`tests/sdd_integration_tests.rs`): New test cases for stage detection and constraint enforcement.

## References

- GitHub Issue: https://github.com/StrayDragon/llman/issues/15

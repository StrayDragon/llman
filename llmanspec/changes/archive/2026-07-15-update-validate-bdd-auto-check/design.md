# Design

## Decision 1: auto-run BDD on validate when configured

**Trade-off**: defaulting to BDD execution makes `validate` slower but prevents false confidence in fast-mode-only validation.

**Alternatives considered**:
1. Keep `--check` opt-in — rejected: the current UX gives false sense of validation completeness
2. Scan step patterns across multiple frameworks — rejected: framework-binding, maintenance burden
3. Auto-run BDD when configured — chosen: BDD frameworks already detect unmatched steps

## Decision 2: BDD-on delta detection (has_spec_files fix)

**Problem discovered during propose**: BDD-on specs have no `requirements` table in `spec.toon`. When a change has only `.feature` deltas (no `spec.toon`), `has_spec_files()` returns `false` → stage stays "draft" forever.

**Fix**: `has_spec_files()` must also recognize directories containing `.feature` files as valid spec dirs. In BDD-on mode the `.feature` file IS the spec delta — `spec.toon` is unnecessary and would break archive (modify_requirement on empty requirements table).

## Implementation approach

Minimal changes to:
- `src/sdd/spec/validation.rs::has_spec_files()` — detect `.feature` files
- `src/sdd/shared/validate.rs` — flip check_mode default
- `src/sdd/command.rs` — add `--no-check`, deprecate `--check`

## Risk

Users with slow BDD suites may find default validate too slow. Mitigation: `--no-check` flag.

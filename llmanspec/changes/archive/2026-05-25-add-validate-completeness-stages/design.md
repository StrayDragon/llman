# Design: Validate Completeness Stages

## Decision: Stage Definition Model

### Options Considered

1. **Linear strict ordering** (proposal → specs → design → tasks): Each stage requires all previous stages to be present.
2. **Flexible with constraints**: Only enforce the critical dependency (design before tasks), allow other stages to be reached out of order.

### Chosen: Option 2 — Flexible with constraints

**Rationale**: The issue explicitly states `tasks.md` depends on `design.md`, but `specs/` and `design.md` can be created independently. A fully linear model would break users who create specs without design (which is valid for simple changes).

### Stage Classification Logic

```
fn determine_stage(change_dir: &Path) -> ChangeStage {
    let has_proposal = change_dir.join("proposal.md").exists();
    let has_specs = change_dir.join("specs").is_dir()
        && has_spec_files(change_dir.join("specs"));
    let has_design = change_dir.join("design.md").exists();
    let has_tasks = change_dir.join("tasks.md").exists();

    match (has_proposal, has_specs, has_design, has_tasks) {
        (true, true, true, true) => ChangeStage::Full,
        (true, true, true, false) => ChangeStage::Designed,
        (true, true, false, _) => ChangeStage::Specified,
        (true, false, _, _) => ChangeStage::Draft,
        _ => ChangeStage::Draft, // proposal missing is handled by existing check
    }
}
```

## Decision: Error vs Warning Levels

| Condition | non-strict | strict |
|-----------|-----------|--------|
| tasks.md exists without design.md | ERROR | ERROR |
| Stage is draft/specified (missing artifacts) | INFO | WARN (escalated from INFO) |
| Stage is designed (missing tasks.md) | INFO | WARN |

The `tasks.md` without `design.md` case is always an ERROR because it violates a hard workflow constraint regardless of strictness.

## Decision: List Output Format

Text format adds a fixed-width `stage` column between name and task status:

```
Active changes:
  c110-add-adaptive-output   full        0/13 tasks    2h ago
  c120-add-dead-code         designed    —             1h ago
  c130-add-nlp-context       specified   —             30m ago
  c200-draft-idea            draft       —             just now
```

JSON format adds a `"stage"` field to each change object.

## Migration

No migration needed. This is purely additive behavior on existing directory scanning.

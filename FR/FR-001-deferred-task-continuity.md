# FR-001: Deferred task continuity — prevent "fire-and-forget" defer links

## Summary

When a task in change X is marked `(defer → Y)`, archiving X succeeds with zero
visibility to Y. The author implementing Y never sees the deferred tasks that X
left behind. This turns defer links into a "fire and forget" mechanism that
silently accumulates work nobody will discover.

## Current Behavior (as of 0.0.42)

### How defer links work today

**`tasks.rs`** — parses `(defer → <target>)` as `TaskStatus::Deferred { target }`.
`LegacyDefer` (old `defer - reason` syntax) and `Pending` are separate states.

**`archive.rs` gate** (line ~80):
```rust
if report.pending > 0 {
    // BLOCK archive — but Deferred tasks are NOT counted in `report.pending`
}
```
`Deferred` tasks bypass the gate entirely. Only `Pending` and `LegacyDefer`
block archiving.

**`validation.rs` `check_tasks_completion`** — validates defer targets exist in
`all_change_ids` ∪ `archived_change_ids`. If the target exists → silent pass.
Zero information about what was deferred.

**`orphans.rs`** — scans for `Deferred { target }` where `target ∉ all_known`.
Only catches *missing* targets, never existing targets with forgotten incoming
work.

### The bug sequence

```
1. c03 has 10 tasks like: - [ ] Migrate notebooks API (defer → c04)
2. `llman sdd validate c03 --strict` → ✅ pass (c04 directory exists)
3. `llman sdd archive run c03` → ✅ pass (report.pending == 0, Deferred ignored)
4. c04 gets implemented by next agent — reads design.md + tasks.md
   → ZERO indication that c03 left 4 items for c04
5. `llman sdd validate c04` → ✅ pass (c04's own tasks are fine)
6. `llman sdd graph` → shows defer edges (but only if you run it with all scope)
```

Deferred tasks vanish into a black hole unless someone manually remembers to check.

## Proposed Solutions

Three layers, from cheapest to most complete:

### P1: Validation on receiving end (low cost, high impact)

Add **incoming defer check** to `check_tasks_completion` (in `validation.rs`):

When validating change Y, scan ALL changes (active + archived) for
`TaskStatus::Deferred { target: Y }`. For each found item, emit an
**Info-level** issue with the source change-id and task text.

```
[INFO] tasks.md: incoming deferred task from c03-add-v2-frontend-eden:
  "notebooks domain: generated import → eden treaty" (defer → c04-add-v2-core-crud)
```

This works when the receiving change is still active. Once the receiving change
gets archived, the check is meaningless (all incoming work is presumed done or
also deferred).

**Implementation sketch:**

```rust
// In validation.rs, near check_tasks_completion:
fn check_incoming_defer(
    active_change_id: &str,
    all_changes_root: &Path,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let changes_dir = all_changes_root.join("changes");
    // Scan both active/ and archive/ for tasks.md files
    for (source_change, is_archive) in iterate_all_changes(&changes_dir) {
        if source_change == active_change_id { continue; }
        let report = tasks::parse_tasks_file(&tasks_path)?;
        for item in &report.items {
            if let TaskStatus::Deferred { target } = &item.status {
                if target == active_change_id {
                    issues.push(ValidationIssue {
                        level: ValidationLevel::Info,
                        path: "tasks.md".into(),
                        message: format!(
                            "Incoming defer from {}: {}",
                            source_change, item.text
                        ),
                    });
                }
            }
        }
    }
    issues
}
```

### P2: `llman sdd incoming <change-id>` subcommand (medium cost)

Expose the reverse-defer scan as a standalone command. Output format matches
`llman sdd orphans`:

```
$ llman sdd incoming c04-add-v2-core-crud

Incoming deferred tasks for c04-add-v2-core-crud (4 tasks from 1 change):

c03-add-v2-frontend-eden (archived):
  - [ ] notebooks domain: generated import → eden treaty
  - [ ] sessions domain: generated import → eden treaty
  - [ ] messages domain: generated import → eden treaty
  - [ ] sources domain: generated import → eden treaty

Tip: When you've addressed these, either:
  1. Complete the task and check it off in the source change's tasks.md
  2. Or re-defer with a new (cancelled — reason) annotation
```

Implement as `src/sdd/shared/incoming.rs` (mirror of `orphans.rs`).

### P3: Archive gate — "incoming acknowledgment" (higher cost, design needed)

Before archiving X, require that every `(defer → Y)` target either:
- (a) has a corresponding "received" marker, OR
- (b) has been archived itself, OR
- (c) explicitly declines the work via `(cancelled — ...)`

This would need a new marker syntax like:
```
- [ ] Migrate notebooks (received ← c03)    // handshake established
```

This is a breaking design change and should be a separate FR.

## Recommendation

Implement **P1** (incoming validation) immediately — it's contained in
`validation.rs`, adds no new commands, and catches the footgun within existing
workflows.

Implement **P2** (`llman sdd incoming`) as a QoL tool for the next release.

Defer **P3** to a future discussion; the design space is open.

## Real-world Impact

In the Crystalith v2 migration, c03 had 10 deferred tasks to 4 different
changes (c04, c09, c12, c14). After archiving c03, those 10 tasks were
invisible unless someone manually inspected c03's archive. The next agent
implementing c04 would have missed 4 domain migration tasks.

## Related Code

- `src/sdd/shared/tasks.rs` — `RE_DEFER_LINKED`, `TaskStatus::Deferred`
- `src/sdd/change/archive.rs` — archive gate at `report.pending > 0`
- `src/sdd/spec/validation.rs` — `check_tasks_completion`
- `src/sdd/shared/orphans.rs` — orphan detection (only missing targets, not existing ones)
- `src/sdd/shared/graph.rs` — `collect_defer_edges` (already does the scan, just not exposed to validation)

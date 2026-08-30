---
name: "llman-sdd-apply-cycle"
description: "Single closed-loop for one change: gate→implement→test→validate→verify→archive→commit. Manual trigger only. Agent MUST NOT auto-invoke."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
disable-model-invocation: true
---

# LLMAN SDD Apply Cycle

End-to-end closed loop for one change (manual). Requires Branch binding and `readyToImplement=true`.

**Manual trigger only**: `/skill:llman-sdd-apply-cycle <change-id>`

## Workflow

### 0) Gate + status
```bash
llman sdd show <change-id> --json --type change
```
> Stage gate: decide from `stage` / `readyToImplement` in `llman sdd show <id> --json --type change`; full decision table lives in llman-sdd-apply.

- Must be on the bound non-default branch.
- If `readyToImplement` is not true → STOP (finish Specs landing or `skip_specs_landing`); **do not** finalize yet.
- Track progress via `tasks.md` checkboxes (or `llman sdd list` task counts); still read `tasks.md`, proposal/design, and live `llmanspec/specs/**` on the bound branch (SSOT).

### 1) Loop: implement → test
For each incomplete task:
1. Implement per task + live specs (minimal diff)
2. Run `tasks[].test` if present
3. On failure, fix and retry (same self-repair budget as `llman-sdd-apply`: cap 8 rounds)
4. Check off `tasks.md` as `[x]`

### 2) Validate
```bash
llman sdd validate <change-id> --strict --no-interactive
```
On failure, fix and retry (same self-repair budget as `llman-sdd-apply`: cap 8 rounds).

### 3) Verify (recommended)
Prefer `llman-sdd-verify` (or equivalent dual-axis self-check). CRITICAL → STOP; do not archive.

### 4) Archive
Prefer:
```bash
llman sdd change finalize <change-id>
```
(dirty tree OK; ff-merge + docs rename; then one `git commit`.)

Fallback: `checkpoint` → `archive` (see `llman-sdd-archive`).

### 5) Commit
```bash
git add -A && git commit -m "<prefix>: <description>"
```

### 6) Optional cleanup
```bash
git branch -d <feature-branch>
```
push / hosting PR only when the user explicitly asks.

## Hard constraints
- **Never ask** "should I continue" unless blocked.
- **Never switch** changes until this one is archived and committed.
- **Retry cap**: self-repair follows `llman-sdd-apply`'s 8-round budget (including the diagnose escalation path).
- **Do not** author `changes/<id>/specs/` or use `change delta`.
- **No default push/PR**.

## Ethics Governance
- `ethics.risk_level`: medium
- `ethics.prohibited_actions`: implement/archive without `readyToImplement`, switching changes early, writing `changes/<id>/specs/`, commit without validation, default push/PR
- `ethics.required_evidence`: `readyToImplement=true`, validate --strict pass, all tasks checked, finalize/archive success
- `ethics.refusal_contract`: after 3 gate/validation failures, report blocker; do not force-archive
- `ethics.escalation_policy`: if changing SDD workflow specs/templates, pause for user confirm before archive

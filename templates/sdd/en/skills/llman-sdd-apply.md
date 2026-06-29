---
name: "llman-sdd-apply"
description: "Implement tasks from an llman SDD change and update tasks.md checkboxes."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Apply

Implement a change by completing `llmanspec/changes/<id>/tasks.md` from top to bottom.

## Steps
1. Use `llman sdd context --task "<goal from proposal>" --paths "<scope from specs>"` to confirm relevant specs.
   - If context is unavailable, rebuild with `llman sdd index rebuild` (default `pageindex` tree index, no model needed) and retry; for the `rag` backend add `--backend rag`.
2. Select the change id:
   - If provided, use it.
   - Otherwise infer from context; if ambiguous, run `llman sdd list --json` and ask the user to choose.
   - Always announce: "Using change: <id>" and how to override.
2. Check prerequisites (authoritative stage gate):
   - Read the change's stage from the authoritative source:
     ```bash
     stage=$(llman sdd show <id> --json --type change | jq -r .stage)
     ```
     (If `jq` is unavailable, parse the `stage` value from the JSON with any tool.)
   - If `stage` is `draft`, the change is not ready to implement → STOP with a guard:
     `draft`: "Change <id> is a draft proposal (proposal.md only). It is not ready to implement. Grow it to at least `spec` stage first with: llman-sdd-continue <id> (proposal → specs → tasks)."
   - `specified`, `designed`, and `full` stages are all ready to implement (tasks.md exists), proceed.
3. Read context files (as applicable):
   - `llmanspec/changes/<id>/proposal.md`
   - `llmanspec/changes/<id>/design.md` (if present)
   - `llmanspec/changes/<id>/tasks.md`
   - `llmanspec/changes/<id>/specs/**`
4. Show status:
   - Progress: "N/M tasks complete"
   - The next 1–3 unchecked tasks (brief)
5. Implement tasks in order:
   - Keep changes minimal and scoped to the current task
   - After completing a task, immediately update its checkbox (`- [ ]` → `- [x]`)
   - If a task is unclear, you hit a blocker, or specs/design don't match reality, STOP and ask what to do next.
{% if bdd_enabled %}
6. **BDD 回归**:
   - 每完成一个 task，运行关联的 BDD 测试确保不回退:
     `{{ bdd_run_command }}`
   - 如有 scenario 由 PASS 变 FAIL，立即停止并报告
{% endif %}
7. When tasks are complete (or when pausing), run validation:
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```
   - If clean, suggest running `llman-sdd-verify`, then archive with `llman sdd archive run <id>`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

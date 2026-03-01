---
name: "llman-sdd-apply"
description: "Implement tasks from an llman SDD change and update tasks.md checkboxes."
metadata:
  llman-template-version: 2
---

# LLMAN SDD Apply

Implement a change by completing `llmanspec/changes/<id>/tasks.md` from top to bottom.

## Steps
1. Select the change id:
   - If provided, use it.
   - Otherwise infer from context; if ambiguous, run `llman sdd-legacy list --json` and ask the user to choose.
   - Always announce: "Using change: <id>" and how to override.
2. Check prerequisites:
   - `llmanspec/changes/<id>/tasks.md` must exist.
   - If missing, suggest `/llman-sdd:continue <id>` (or `/llman-sdd:ff <id>`) to create planning artifacts, then STOP.
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
   - If a task is unclear, you hit a blocker, or specs/design don’t match reality, STOP and ask what to do next.
6. When tasks are complete (or when pausing), run validation:
   ```bash
   llman sdd-legacy validate <id> --strict --no-interactive
   ```
   - If clean, suggest `/llman-sdd:verify <id>` and `/llman-sdd:archive <id>`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

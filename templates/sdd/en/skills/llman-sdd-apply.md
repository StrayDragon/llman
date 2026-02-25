---
name: "llman-sdd-apply"
description: "Implement tasks from an llman SDD change and update tasks.md checkboxes."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Apply

Use this skill to implement `tasks.md` for a change.

## Steps
1. Select the change id:
   - If provided, use it.
   - Otherwise run `llman sdd list --json` and ask the user to choose.
2. Read context files (as applicable):
   - `llmanspec/changes/<id>/proposal.md`
   - `llmanspec/changes/<id>/design.md` (if present)
   - `llmanspec/changes/<id>/tasks.md`
   - `llmanspec/changes/<id>/specs/**`
3. Implement tasks in order:
   - Keep changes minimal and scoped to the current task
   - After completing a task, update its checkbox (`- [ ]` → `- [x]`)
4. If a task is unclear or you hit a blocker, STOP and ask the user what to do next.
5. When tasks are complete (or when pausing), run validation:
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

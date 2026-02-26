<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/llman-sdd/apply.md (copied 2026-02-09) -->

Implement tasks for a change in `llmanspec/changes/<id>/`.

**Input**: Optionally specify a change id (e.g., `/llman-sdd:apply add-auth`). If omitted, infer from context; if ambiguous, prompt the user to choose.

**Steps**

1. **Select the change**

   If an id is provided, use it. Otherwise:
   - If the conversation clearly references a change id, use it.
   - Else run `llman sdd list --json`, show the most recent changes, and ask the user to pick one.

   Always announce: "Using change: <id>" and how to override (e.g., `/llman-sdd:apply <other>`).

2. **Check prerequisites**

   Ensure these exist:
   - `llmanspec/changes/<id>/tasks.md`

   If missing, suggest using `/llman-sdd:continue <id>` (or `/llman-sdd:ff <id>`) to create planning artifacts first, then STOP.

3. **Read context artifacts**

   Read:
   - `llmanspec/changes/<id>/proposal.md` (if present)
   - `llmanspec/changes/<id>/specs/*/spec.md` (all delta specs)
   - `llmanspec/changes/<id>/design.md` (if present)
   - `llmanspec/changes/<id>/tasks.md`

4. **Show current progress**

   Display:
   - Progress: "N/M tasks complete"
   - The next 1–3 pending tasks (brief)

5. **Implement tasks (loop until done or blocked)**

   For each pending task:
   - Announce the task you’re working on
   - Make the code changes required (keep scope minimal)
   - Mark task complete in `tasks.md`: `- [ ]` → `- [x]`

   Pause if:
   - A task is unclear → ask the user before proceeding
   - Implementation reveals a spec/design mismatch → propose updating artifacts
   - You hit an error/blocker → report and ask for direction

6. **On completion**

   When all tasks are checked:
   - Suggest `/llman-sdd:verify <id>` (optional but recommended)
   - Suggest `/llman-sdd:archive <id>` to archive and update main specs

**Guardrails**
- Keep edits minimal and focused on one task at a time
- Update the checkbox immediately after each task is completed

**Options:**
1. <option 1>
2. <option 2>
3. Other approach

What would you like to do?
```

**Guardrails**
- Keep going through tasks until done or blocked
- Always read context files before starting (from the apply instructions output)
- If task is ambiguous, pause and ask before implementing
- If implementation reveals issues, pause and suggest artifact updates
- Keep code changes minimal and scoped to each task
- Update task checkbox immediately after completing each task
- Pause on errors, blockers, or unclear requirements - don't guess
- Use contextFiles from CLI output, don't assume specific file names

**Fluid Workflow Integration**

This skill supports the "actions on a change" model:

- **Can be invoked anytime**: Before all artifacts are done (if tasks exist), after partial implementation, interleaved with other actions
- **Allows artifact updates**: If implementation reveals design issues, suggest updating artifacts - not phase-locked, work fluidly

{{ unit("skills/structured-protocol") }}

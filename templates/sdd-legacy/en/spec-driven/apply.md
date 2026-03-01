<!-- llman-template-version: 2 -->
<!-- source: OpenSpec templates/en/llman-sdd/apply.md (copied 2026-02-09) -->

Implement tasks for a change in `llmanspec/changes/<id>/`.

**Input**: Optionally specify a change id after `/llman-sdd:apply` (e.g., `/llman-sdd:apply add-auth`). If omitted, infer from context; if ambiguous, prompt the user to choose.

**Steps**

1. **Select the change**

   If an id is provided, use it. Otherwise:
   - If the conversation clearly references a change id, use it.
   - Else run `llman sdd-legacy list --json`, show the most recent changes, and ask the user to pick one.

   Always announce: "Using change: <id>" and how to override (e.g., `/llman-sdd:apply <other>`).

2. **Check prerequisites**

   Ensure these exist:
   - `llmanspec/changes/<id>/tasks.md`

   If missing, suggest using `/llman-sdd:continue <id>` (or `/llman-sdd:ff <id>`) to create planning artifacts first, then STOP.

3. **Read context artifacts**

   Read what exists under `llmanspec/changes/<id>/`:
   - `proposal.md` (if present)
   - `specs/*/spec.md` (all delta specs)
   - `design.md` (if present)
   - `tasks.md`

4. **Show current progress**

   Display:
   - Progress: "N/M tasks complete"
   - The next 1–3 pending tasks (brief)

5. **Implement tasks (loop until done or blocked)**

   For each unchecked task:
   - Announce the task you’re working on
   - Make the code changes required (keep scope minimal)
   - Mark the task complete in `tasks.md`: `- [ ]` → `- [x]`

   STOP and ask if:
   - A task is unclear or missing context
   - Implementation reveals a spec/design mismatch (suggest updating artifacts first)
   - You hit an error/blocker

6. **On completion**

   When all tasks are checked:
   - Run `llman sdd-legacy validate <id> --strict --no-interactive`
   - Suggest `/llman-sdd:verify <id>` (optional but recommended)
   - Suggest `/llman-sdd:archive <id>` to archive and update main specs

**Output**

Summarize:
- Change id used
- Tasks completed this session
- Remaining tasks (if any) and the suggested next step

**Guardrails**
- Keep edits minimal and focused on one task at a time
- Always read the current artifacts before editing files
- Don’t invent evidence — cite file paths and concrete observations
- Use actual repo paths; don’t assume file names beyond what exists
- Update the checkbox immediately after each task is completed

{{ unit("skills/structured-protocol") }}

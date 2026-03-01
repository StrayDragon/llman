<!-- llman-template-version: 2 -->
<!-- source: OpenSpec templates/en/llman-sdd/new.md (copied 2026-02-09) -->

Start a new change in llman SDD (directory only; no artifacts yet).

**Input**: The argument after `/llman-sdd:new` is either:
- a change id (kebab-case), or
- a short description (derive an id and confirm with the user).

**Steps**

1. **Determine the change id**

   If an id is provided, use it. Otherwise:
   - Ask what the user wants to build/fix.
   - Propose a kebab-case id (e.g., "add user authentication" → `add-user-auth`).
   - Confirm the id before creating any directories.

   **STOP** if the id is invalid or ambiguous.

2. **Ensure the project is initialized**

   Check that `llmanspec/` exists in the repo root.
   - If missing: tell the user to run `llman sdd-legacy init` first, then STOP.

3. **Create the change directory (no artifacts)**

   Create:
   - `llmanspec/changes/<id>/`
   - `llmanspec/changes/<id>/specs/`

   If the change already exists, suggest using `/llman-sdd:continue <id>` instead.

4. **STOP and wait for user direction**

**Output**

After completing the steps, summarize:
- Change id and location (`llmanspec/changes/<id>/`)
- Current state (no artifacts yet)
- Prompt: "Ready to create the first artifact? Run `/llman-sdd:continue <id>`."
- Alternative: "Want everything created now? Run `/llman-sdd:ff <id>`."

**Guardrails**
- Do NOT implement application code
- Do NOT create any change artifacts yet (proposal/specs/design/tasks) — `/llman-sdd:continue` or `/llman-sdd:ff` will do that
- Do NOT guess an id; if invalid (not kebab-case), ask for a valid id

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}

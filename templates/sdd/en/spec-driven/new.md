<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/opsx/new.md (copied 2026-02-09) -->

Start a new change in llman SDD using the OPSX action-based workflow.

**Input**: The argument after `/opsx:new` is the change id (kebab-case), OR a description of what the user wants to build.

**Steps**

1. **If no clear input, ask what they want to build**

   Ask:
   > "What change do you want to work on? Describe what you want to build or fix."

   From their description, derive a kebab-case id (e.g., "add user authentication" → `add-user-auth`).

   **IMPORTANT**: Do NOT proceed without understanding what the user wants to build.

2. **Ensure the project is initialized**

   Check that `llmanspec/` exists in the repo root.
   - If missing: tell the user to run `llman sdd init` first, then STOP.

3. **Create the change directory (no artifacts yet)**

   Create:
   - `llmanspec/changes/<id>/`
   - `llmanspec/changes/<id>/specs/`

   If the change already exists, suggest using `/opsx:continue <id>` instead.

4. **STOP and wait for user direction**

**Output**

After completing the steps, summarize:
- Change id and location (`llmanspec/changes/<id>/`)
- Current state (no artifacts yet)
- Prompt: "Ready to create the first artifact? Run `/opsx:continue <id>` (or just tell me what to do next)."

**Guardrails**
- Do NOT implement application code
- Do NOT create any change artifacts yet (proposal/specs/design/tasks) — `/opsx:continue` or `/opsx:ff` will do that
- If the id is invalid (not kebab-case), ask for a valid id

<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/opsx/ff.md (copied 2026-02-09) -->

Fast-forward through planning — generate all artifacts needed to start implementation in llman SDD.

**Input**: The argument after `/opsx:ff` is the change id (kebab-case), OR a description of what the user wants to build.

**Steps**

1. **If no clear input, ask what they want to build**

   Ask:
   > "What change do you want to work on? Describe what you want to build or fix."

   Derive a kebab-case id (e.g., "add user authentication" → `add-user-auth`).

2. **Ensure the project is initialized**

   Check that `llmanspec/` exists. If missing, tell the user to run `llman sdd init` first, then STOP.

3. **Create the change directory**

   Create `llmanspec/changes/<id>/` and `llmanspec/changes/<id>/specs/` if missing.

   If the change already exists, ask whether to:
   - Continue and fill missing artifacts (recommended), or
   - Use a different id.

4. **Create artifacts (spec-driven)**

   Create these artifacts in order:

   a) `proposal.md`
   - Fill in Why / What Changes / Capabilities / Impact.
   - If scope is unclear, ask 1–2 clarifying questions before writing.

   b) `specs/<capability>/spec.md` (for each capability)
   - For each capability listed in the proposal, create a delta spec at:
     `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - Use `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` and at least one `#### Scenario:` per requirement.

   c) `design.md` (optional)
   - If the change spans multiple systems, is risky, or needs tradeoffs: create `design.md`.
   - Otherwise, skip design (or add a short stub only if the user wants it).

   d) `tasks.md`
   - Break down implementation into small, checkable tasks.
   - Include validation commands (e.g., `just check`, `llman sdd validate <id> --strict --no-interactive`).

5. **Validate and hand off to implementation**

   Suggest running:
   - `llman sdd validate <id> --strict --no-interactive`

   Then prompt:
   - "Ready for implementation. Run `/opsx:apply <id>`."

**Guardrails**
- Do NOT implement application code
- Keep artifacts minimal and consistent with the requested change
- Ask before making breaking-scope assumptions

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}
{{region: templates/sdd/en/skills/shared.md#future-planning}}

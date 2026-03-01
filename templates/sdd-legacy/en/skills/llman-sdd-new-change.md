---
name: "llman-sdd-new-change"
description: "Create a new change proposal and delta specs."
metadata:
  llman-template-version: 2
---

# LLMAN SDD New Change

Create a new change with planning artifacts (proposal + delta specs + tasks; design optional).

## Steps
1. Determine the change id and scope (kebab-case, verb prefix: `add-`, `update-`, `remove-`, `refactor-`).
   - If the user only gave a description, ask 1–3 clarifying questions, then propose an id and confirm it.
2. Ensure the project is initialized:
   - `llmanspec/` must exist; if missing, tell the user to run `llman sdd-legacy init`, then STOP.
3. Create `llmanspec/changes/<change-id>/` and `llmanspec/changes/<change-id>/specs/`.
   - If the change already exists, STOP and suggest `llman-sdd-continue` (or `/llman-sdd:continue <id>`).
4. Create artifacts under `llmanspec/changes/<change-id>/`:
   - `proposal.md` (Why / What Changes / Capabilities / Impact)
   - `specs/<capability>/spec.md` for each capability using:
     - `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`
     - at least one `#### Scenario:` per requirement
   - `design.md` only when tradeoffs/migrations matter
   - `tasks.md` as an ordered checklist (include validation commands)
5. Validate: `llman sdd-legacy validate <change-id> --strict --no-interactive`.
6. Hand off to implementation: suggest `llman-sdd-apply` (or `/llman-sdd:apply <id>`).

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}

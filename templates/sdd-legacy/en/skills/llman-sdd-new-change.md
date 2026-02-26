---
name: "llman-sdd-new-change"
description: "Create a new change proposal and delta specs."
metadata:
  llman-template-version: 1
---

# LLMAN SDD New Change

Use this skill when you need to introduce a new capability, breaking change, or architecture shift.

## Steps
1. Pick a unique change id (kebab-case, verb prefix: `add-`, `update-`, `remove-`, `refactor-`).
2. Create `llmanspec/changes/<change-id>/` with:
   - `proposal.md`
   - `tasks.md`
   - optional `design.md`
3. For each affected capability, add `specs/<capability>/spec.md` using:
   - `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`
   - at least one `#### Scenario:` per requirement
4. Validate: `llman sdd validate <change-id> --strict --no-interactive`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}

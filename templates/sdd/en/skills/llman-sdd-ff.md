---
name: "llman-sdd-ff"
description: "Fast-forward: create proposal/specs/design/tasks for a change in one pass."
metadata:
  llman-template-version: 2
---

# LLMAN SDD Fast-Forward (FF)

Use this skill to create **all** artifacts for a new change quickly (proposal → specs → design (optional) → tasks).

## Steps
1. Ask the user for:
   - A short description of the change
   - A preferred change id (or derive one; kebab-case, verb prefix)
   - The capability/capabilities impacted (to name `specs/<capability>/`)
   - Confirm the final id before creating any directories.
2. Ensure the project is initialized:
   - `llmanspec/` must exist; if missing, tell the user to run `llman sdd init`, then STOP.
3. If `llmanspec/changes/<id>/` already exists, ask whether to:
   - Continue and fill missing artifacts (recommended), or
   - Use a different id.
   Do NOT overwrite existing artifacts without explicit confirmation.
4. Create artifacts under `llmanspec/changes/<id>/`:
   - `proposal.md`
   - `specs/<capability>/spec.md` (at least one)
   - `design.md` (only if needed)
   - `tasks.md` (ordered, small, verifiable tasks including validation)
5. Validate:
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```
6. Show a short status summary and suggest next actions (`llman-sdd-apply` or `/llman-sdd:apply`).

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}

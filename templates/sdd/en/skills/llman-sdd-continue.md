---
name: "llman-sdd-continue"
description: "Continue an existing llman SDD change by creating the next artifact."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Continue

Use this skill to continue an existing change and create the next missing artifact.

## Steps
1. Identify the change id:
   - If provided by the user, use it.
   - Otherwise run `llman sdd list --json` and ask which change to continue.
2. Read the change directory: `llmanspec/changes/<id>/`.
3. Determine the next artifact to create (in order):
   1) `proposal.md`
   2) `specs/<capability>/spec.md` (one folder per capability)
   3) `design.md` (only if design tradeoffs matter)
   4) `tasks.md`
4. Create exactly ONE missing artifact under `llmanspec/changes/<id>/`.
5. If all artifacts already exist, suggest next actions:
   - Implement: `llman-sdd-apply`
   - Validate: `llman sdd validate <id> --strict --no-interactive`
   - Archive (when ready): `llman sdd archive <id>`

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}
{{region: templates/sdd/en/skills/shared.md#validation-hints}}

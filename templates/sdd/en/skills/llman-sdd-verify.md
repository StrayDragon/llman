---
name: "llman-sdd-verify"
description: "Verify implementation matches llman SDD specs/design and propose fixes."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Verify

Use this skill to verify that the implementation matches the change’s artifacts.

## Steps
1. Select the change id (or ask the user to pick from `llman sdd list --json`).
2. Read:
   - Delta specs under `llmanspec/changes/<id>/specs/`
   - `proposal.md` and `design.md` if present
   - `tasks.md` to understand what was implemented
3. Compare artifacts vs code:
   - Identify mismatches (missing behavior, wrong behavior, missing tests/docs)
   - Suggest minimal fixes or artifact updates
4. Run the repo’s verification commands as appropriate (tests, lint, etc).
5. If everything aligns, suggest archive: `llman sdd archive <id>`.

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}

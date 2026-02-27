---
name: "llman-sdd-verify"
description: "Verify implementation matches llman SDD specs/design and propose fixes."
metadata:
  llman-template-version: 2
---

# LLMAN SDD Verify

Use this skill to verify that the implementation matches the change’s artifacts.

## Steps
1. Select the change id (or ask the user to pick from `llman sdd list --json`).
2. Run a fast validation gate:
   - `llman sdd validate <id> --strict --no-interactive`
3. Read:
   - Delta specs under `llmanspec/changes/<id>/specs/`
   - `proposal.md` and `design.md` if present
   - `tasks.md` to understand what was implemented
4. Compare artifacts vs code:
   - Identify mismatches (missing behavior, wrong behavior, missing tests/docs)
   - Suggest minimal fixes or artifact updates
5. Produce a short report:
   - **CRITICAL** (must fix before archive)
   - **WARNING** (should fix)
   - **SUGGESTION** (nice to have)
6. If CRITICAL exists, suggest `llman-sdd-apply` (or `/llman-sdd:apply <id>`). If clean, suggest archive: `llman sdd archive <id>`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

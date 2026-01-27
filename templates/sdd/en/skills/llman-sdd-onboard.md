---
name: "LLMAN SDD Onboard"
description: "Onboard to the llman spec-driven workflow."
---

# LLMAN SDD Onboard

Use this skill to onboard to llman SDD in a repository.

## Steps
1. Read `llmanspec/AGENTS.md` and `llmanspec/project.md`.
2. Check current changes and specs.
3. Follow the proposal -> implement -> archive workflow.

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}

## Notes
- `llmanspec/config.yaml` controls locale and skills paths.
- Locale affects templates/skills only; CLI stays English.
- Refresh skills with `llman sdd update-skills`.

{{region: templates/sdd/en/skills/shared.md#validation-hints}}

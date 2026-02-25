---
name: "llman-sdd-onboard"
description: "Onboard to the llman SDD workflow in a repository."
metadata:
  llman-template-version: 1
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

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}

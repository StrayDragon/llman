---
name: "llman-sdd-onboard"
description: "Onboard to the llman SDD workflow in a repository."
---

# LLMAN SDD Onboard

Use this skill to onboard to llman SDD in a repository.

## Steps
1. Read `llmanspec/config.yaml` for project context, conventions, and rules.
2. Check current changes and specs.
3. Follow the proposal -> implement -> archive workflow.

{{ unit("skills/sdd-commands") }}

## Notes
- `llmanspec/config.yaml` holds project context, rules, locale, and skills paths.
- Locale affects templates/skills only; CLI stays English.
- Refresh skills with `llman sdd update-skills`.

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

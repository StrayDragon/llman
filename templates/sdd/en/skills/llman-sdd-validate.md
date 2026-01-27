---
name: "llman-sdd-validate"
description: "Validate llmanspec changes and specs with actionable fixes."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Validate

Use this skill to validate change/spec format and staleness.

## Steps
1. Validate one item: `llman sdd validate <id>`.
2. Validate all: `llman sdd validate --all` (or `--changes` / `--specs`).
3. Use `--strict` and `--no-interactive` for CI-like checks.

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}

{{region: templates/sdd/en/skills/shared.md#validation-hints}}

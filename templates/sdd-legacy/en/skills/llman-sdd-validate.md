---
name: "llman-sdd-validate"
description: "Validate llmanspec changes and specs with actionable fixes."
metadata:
  llman-template-version: 2
---

# LLMAN SDD Validate

Use this skill to validate change/spec format and staleness.

## Steps
1. Validate one item: `llman sdd-legacy validate <id>`.
2. Validate all: `llman sdd-legacy validate --all` (or `--changes` / `--specs`).
3. Use `--strict` and `--no-interactive` for CI-like checks.
4. If validation fails, summarize the errors and propose minimal, concrete fixes.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

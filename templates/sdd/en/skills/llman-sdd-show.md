---
name: "llman-sdd-show"
description: "Inspect llmanspec changes and specs quickly."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Show

Use this skill to inspect changes, specs, and JSON output.

## Steps
1. List items: `llman sdd list` or `llman sdd list --specs`.
2. Show details: `llman sdd show <id>`.
3. Disambiguate with `--type change|spec` when needed.
4. Use `--json` for structured output.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

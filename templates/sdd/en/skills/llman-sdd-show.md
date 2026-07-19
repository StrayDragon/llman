---
name: "llman-sdd-show"
description: "Inspect llmanspec changes and specs quickly."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Show

Use this skill to inspect changes, specs, and JSON output.

## Steps
1. List items: `llman sdd list` or `llman sdd list --specs`.
2. If the id is unknown or ambiguous, show the list and ask the user to pick.
3. Show details: `llman sdd show <id>`.
4. Disambiguate with `--type change|spec` when needed.
5. Use `--json` for structured output.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

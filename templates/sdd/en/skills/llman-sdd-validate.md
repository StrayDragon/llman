---
name: "llman-sdd-validate"
description: "Validate llmanspec changes and specs with actionable fixes."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Validate

Use this skill to validate change/spec format and staleness.

## Steps
1. Validate one item: `llman sdd validate <id>`.
2. Validate all: `llman sdd validate --all` (or `--changes` / `--specs`).
3. Use `--strict` and `--no-interactive` for CI-like checks.
4. If validation fails, summarize the errors and propose minimal, concrete fixes.
{% if bdd_enabled %}
5. **BDD checks (Partitioned SSOT)**:
   - Validate `.feature` Gherkin and `@req` / dual-write gates.
   - `.feature` is the harness authority — maintain executable GWT there; `solidify` is a consistency gate (not projection overwrite).
   - `llman sdd validate --specs` runs `bdd.run_command` by default.
   - Use `list --specs --json` for `morphology` (includes `dualWriteCount`).
{% endif %}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

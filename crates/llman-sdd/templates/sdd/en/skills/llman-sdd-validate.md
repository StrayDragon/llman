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
5. **BDD checks (Git-native Partitioned SSOT)**:
   - Validate live `.feature` Gherkin and `@req` / dual-write gates on the **bound branch** (Branch binding required).
   - `.feature` is the harness authority — executable GWT lives only in live `.feature` (no solidify; no `feature_delta` / `change delta`).
   - Change lifecycle gates: `change start` / `attach` (Branch binding), `finalize` (close-out; auto commit `archive(sdd): <id>`, `--no-commit` to skip) / `diff` (read-only). `change checkpoint` is removed (no mid-flight archive point; `change finalize` does not require a clean tree).
   - `llman sdd validate --specs` runs `bdd.run_command` by default.
   - Use `list --specs --json` for `morphology` (includes `dualWriteCount`).
   - Change JSON status fields: `stage` (draft/designed/planned/full) / `specsLanded` / `needsSpecsChange` / `readyToImplement` (`show --json`).
{% endif %}

> For command details run `llman sdd <cmd> --help`; the CLI is the command reference — skills embed no command tables.
> "Spec" here = a `.feature` file under this project's `llmanspec/specs/`; run `llman sdd list --specs` or `llman sdd show <capability>`.

{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}

---
name: "llman-sdd-specs-compact"
description: "Human-triggered maintenance tool. Compacts and deduplicates llman SDD specs after many archived changes — merges redundant requirements and scenarios while preserving all normative behavior. NOT part of the regular pipeline: only run when the user explicitly asks to compact specs."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Specs Compact

Use this skill to compact specs without changing normative behavior.

## Pipeline Position

```mermaid
flowchart LR
    archive["llman-sdd-archive<br/>After archiving"] --> compact
    compact["📎 llman-sdd-specs-compact<br/>Compact specs (maintenance)"]

    style compact fill:#e8f4e8,stroke:#28a745,stroke-width:2px
```

> 📎 Maintenance tool, typically run after accumulating many archives. For daily development → `llman-sdd-propose` (Branch binding + Specs landing) / `llman-sdd-apply` (requires `readyToImplement`).

## Context
- Specs grow bloated with duplicate requirements/scenarios as changes accumulate.
- Compaction must remain verifiable and regressible.
- When archive history is too large, it interferes with compaction review and navigation.

## Goal
- Identify and merge redundant requirements/scenarios.
- Form a more compact and maintainable spec structure.

## Constraints
- Don't delete normative behavior without explicit replacement.
- Try to keep requirement titles stable.
- Each retained requirement must have at least one valid scenario.
- **Editing live `llmanspec/specs/**` requires a change**: Branch binding first (`change start` / `attach`), then commit on the bound branch (Specs landing style); **never** compact-rewrite live specs on the default branch.

## Workflow
1. Inventory current specs (`llman sdd list --specs`).
2. If archived history is large, run archive freeze first:
   - Preview: `llman sdd archive freeze --dry-run`
   - Execute: `llman sdd archive freeze --before <YYYY-MM-DD> --keep-recent <N>`
3. Identify overlapping items across capabilities.
4. Produce a compaction plan (canonical requirements + keep/merge/remove decisions + migration notes).
5. Execute and validate (`llman sdd validate --specs --strict --no-interactive`).

## Decision Policy
- Prefer merging when two requirements are semantically equivalent.
- Only extract shared spec text when reference relationships are clear.
- When archive directory is noisy, suggest freezing first before compacting.
- If compaction would change external behavior, pause and ask the user first.

## Output Contract
- Output compaction plan grouped by capability.
- Include: keep/merge/remove decisions with rationale.
- Include validation commands and expected results.

> 💡 After maintenance, new work goes through the normal pipeline: `llman-sdd-propose` (Branch binding + Specs landing) → `llman-sdd-apply` (requires `readyToImplement`) → `llman-sdd-verify` → `llman-sdd-archive`.

> For command details run `llman sdd <cmd> --help`; the CLI is the command reference — skills embed no command tables (r139).

{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}

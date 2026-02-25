---
name: "llman-sdd-specs-compact"
description: "Compact and refactor llman SDD specs while preserving normative behavior."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Specs Compact

Use this skill to reduce specs bloat while preserving MUST/Scenario behavior.

## Context
- Specs can grow and duplicate requirements across capabilities.
- Compaction must preserve behavior and testability.

## Goal
- Identify redundant requirement/scenario text.
- Produce a compact structure with clear retained canonical requirements.

## Constraints
- Never remove normative behavior without explicit replacement.
- Keep requirement headers stable when possible.
- Every retained requirement must keep at least one valid scenario.

## Workflow
1. Inventory current specs (`llman sdd list --specs`).
2. Map overlap candidates across capabilities.
3. Propose canonical requirements and migration notes.
4. Validate impacted specs (`llman sdd validate --specs --strict --no-interactive`).

## Decision Policy
- Prefer deduplication when two requirements are semantically equivalent.
- Prefer extraction into shared capability text only when references remain clear.
- Stop and ask when compaction would alter external behavior.

## Output Contract
- Provide a compacting plan grouped by capability.
- Include: keep/merge/remove decisions and rationale.
- Include validation commands and expected outcomes.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

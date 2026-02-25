<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/opsx/sync.md (copied 2026-02-09) -->

Sync delta specs from an active change into main specs **without archiving** the change.

This is a manual, reproducible protocol: read delta specs under `llmanspec/changes/<id>/specs/` and apply them to `llmanspec/specs/`.

**Input**: Optionally specify a change id after `/opsx:sync` (e.g., `/opsx:sync add-auth`). If omitted, infer from context; if ambiguous, prompt the user to choose.

**Steps**

1. **Select the change**

   If an id is provided, use it. Otherwise run `llman sdd list --json` and ask the user which change to sync.

2. **Find delta specs**

   Look for delta spec files at:
   - `llmanspec/changes/<id>/specs/<capability>/spec.md`

   If none exist, report it and STOP.

3. **Apply deltas to main specs**

   For each `<capability>` delta:
   - Read the delta spec.
   - Read (or create) the main spec:
     - `llmanspec/specs/<capability>/spec.md`

   Apply changes by section:
   - `## ADDED Requirements`: add missing requirements
   - `## MODIFIED Requirements`: update existing requirements/scenarios
   - `## REMOVED Requirements`: remove requirements
   - `## RENAMED Requirements`: rename requirements (FROM/TO pairs)

   If you create a new main spec file, include required YAML frontmatter and the required sections:
   - YAML frontmatter with `llman_spec_valid_scope`, `llman_spec_valid_commands`, `llman_spec_evidence`
   - `## Purpose`
   - `## Requirements`

4. **Validate**

   Run:
   - `llman sdd validate --specs --strict --no-interactive`

5. **Summarize**

   Summarize which specs changed and what was added/modified/removed/renamed.

**Guardrails**
- Preserve existing content not mentioned in the delta unless the user asks otherwise
- If anything is unclear, ask before editing main specs

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}

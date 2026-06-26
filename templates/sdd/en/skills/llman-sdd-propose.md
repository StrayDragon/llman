---
name: "llman-sdd-propose"
description: "Propose a new change and generate planning artifacts in one pass."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Propose

Create a new change and generate all planning artifacts in one pass (proposal + delta specs + tasks; design optional), then validate and suggest next actions.

## Steps
1. Assess change scale (triage):
   - **Behavioural contract change** (modify MUST/SHALL, change external behaviour) → full SDD workflow
   - **Implementation change** (refactor, typo, perf) → quick path via `llman-sdd-quick`
   - **Meta-spec change** (SDD templates/process) → full SDD workflow
   - When uncertain, choose full SDD (conservative).
2. Use `llman sdd context --task "<goal>" --paths "<scope>"` to find relevant specs.
   - If context unavailable, start `llman sdd index rebuild --run-async` in background and continue.
3. Gather input:
   - A short description of the change
   - A change id (or derive one; kebab-case, verb prefix: `add-`, `update-`, `remove-`, `refactor-`)
   - The impacted capability/capabilities (to name `specs/<capability>/`)
   - Confirm the final id before writing files
2. Ensure the project is initialized:
   - `llmanspec/` must exist; if missing, tell the user to run `llman sdd init`, then STOP.
3. Create `llmanspec/changes/<change-id>/` and `llmanspec/changes/<change-id>/specs/`.
   - If the change already exists, STOP and suggest `llman-sdd-continue`.
4. Create artifacts under `llmanspec/changes/<change-id>/`:
   - `proposal.md` (Why / What Changes / Capabilities / Impact)
   - `specs/<capability>/spec.toon` for each capability (a standalone TOON document, one per file):
     - Prefer generating via authoring helpers so the TOON payload is well-formed:
       - `llman sdd delta skeleton <change-id> <capability>`
       - `llman sdd delta add-op ...`
       - `llman sdd delta add-scenario ...`
     - Include at least one `add_requirement`/`modify_requirement` op (statement MUST contain MUST/SHALL) and at least one matching op scenario row
   - `design.md` only when tradeoffs/migrations matter
   - `tasks.md` as an ordered checklist (include validation commands)
5. Validate:
   ```bash
   llman sdd validate <change-id> --strict --no-interactive
   ```
   This MUST pass before proceeding. If TOON parse errors appear, fix quoting:
   values containing commas/colons/brackets must be double-quoted in tabular rows.
6. Summarize what was created and suggest `llman-sdd-apply` for implementation.

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}

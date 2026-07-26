---
name: "llman-sdd-new-change"
description: "Create a new change proposal with planning artifacts."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD New Change

Create a new change with planning artifacts (proposal + tasks; design optional). Edit live specs on a feature branch.

## Steps
1. Determine the change id and scope (kebab-case, verb prefix: `add-`, `update-`, `remove-`, `refactor-`).
   - If the user only gave a description, ask 1–3 clarifying questions, then propose an id and confirm it.
2. Ensure the project is initialized:
   - `llmanspec/` must exist; if missing, tell the user to run `llman sdd init`, then STOP.
3. Create `llmanspec/changes/<change-id>/` (no `specs/` subdirectory).
   - If the change already exists, STOP and suggest `llman-sdd-continue`.
4. Create artifacts under `llmanspec/changes/<change-id>/`:
   - `proposal.md` (Why / What Changes / Capabilities / Impact)
   - `design.md` only when tradeoffs/migrations matter
   - `tasks.md` as an ordered checklist (include validation commands)
   - On a non-default feature branch, edit live `llmanspec/specs/<capability>/spec.toon` (+ `*.feature` with `@req` when `bdd:` configured); then `llman sdd change start <change-id>` (or `change attach`). Do **not** write under `changes/<id>/specs/` or create `*.feature.delta.toon`.
5. Validate: `llman sdd validate <change-id> --strict --no-interactive`.
   This MUST pass before proceeding. If TOON parse errors appear, fix quoting:
   values containing commas/colons/brackets must be double-quoted in tabular rows.
6. Hand off to implementation: suggest `llman-sdd-apply`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

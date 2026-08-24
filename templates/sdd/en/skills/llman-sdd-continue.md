---
name: "llman-sdd-continue"
description: "Continue an existing llman SDD change by creating the next artifact."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Continue

Use this skill to continue an existing change and create the next missing artifact.

## Steps
1. Identify the change id:
   - If provided by the user, use it.
   - Otherwise run `llman sdd list --json` and ask which change to continue.
   - Always announce: "Using change: <id>".
2. Read the change directory: `llmanspec/changes/<id>/`.
{{ unit("skills/stage-guard") }}
3. Determine the next artifact to create (in order):
   1) `proposal.md`
   2) `design.md` (only if design tradeoffs matter)
   3) `tasks.md`
   4) `llman sdd change start <id>` (or `change attach <id>` if the branch already exists) — Branch binding
   5) Edit live `llmanspec/specs/<capability>/<capability>.feature` on the **bound branch** and commit — Specs landing (or set `skip_specs_landing: true` when there is no contract edit)
4. Create exactly ONE missing artifact (or one live spec/feature edit on the bound branch).
   - Do NOT implement application code in continue mode.
   - Do NOT create `*.feature.delta.toon`, `spec.toon`, or files under `changes/<id>/specs/`.
   - Do NOT edit shared `llmanspec/specs/**` before start/attach.
5. If all artifacts already exist, suggest next actions from `llman sdd show <id> --json`:
   - `readyToImplement=false` → finish Specs landing (or `skip_specs_landing`); do **not** suggest apply yet
   - `readyToImplement=true` → Implement: `llman-sdd-apply`
   - After verify → Archive: `llman-sdd-archive`
   - Validate: `llman sdd validate <id> --strict --no-interactive`
   - Review: `llman sdd change diff <id>` (read-only)

{{ unit("skills/git-native-flow") }}
{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

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
   - Determine the stage authoritatively:
     ```bash
     stage=$(llman sdd show <id> --json --type change | jq -r .stage)
     ```
     (If `jq` is unavailable, parse the `stage` value from the JSON with any tool.)
   - If `stage` is `draft` (proposal.md only), explicitly tell the user: "This is a draft proposal. Grow it until apply-ready (design → tasks → `change start`/`attach` → Specs landing, with `readyToImplement=true`); a draft cannot be applied or verified directly." If proposal+design+tasks already exist but stage is still `draft`, the next step is `llman sdd change start <id>` on a clean default branch (or create a branch then `change attach`) — do NOT create `changes/<id>/specs/`, and do NOT edit live `llmanspec/specs/**` on the default branch first.
3. Determine the next artifact to create (in order):
   1) `proposal.md`
   2) `design.md` (only if design tradeoffs matter)
   3) `tasks.md`
   4) `llman sdd change start <id>` (or `change attach <id>` if the branch already exists) — Branch binding
   5) Edit live `llmanspec/specs/<capability>/spec.toon` (+ `*.feature` when `bdd:` configured) on the **bound branch** and commit — Specs landing (or set `skip_specs_landing: true` when there is no contract edit)
4. Create exactly ONE missing artifact (or one live spec/feature edit on the bound branch).
   - Do NOT implement application code in continue mode.
   - Do NOT create `*.feature.delta.toon` or files under `changes/<id>/specs/`.
   - Do NOT edit shared `llmanspec/specs/**` before start/attach.
5. If all artifacts already exist, suggest next actions:
   - If `llman sdd show <id> --json` has `readyToImplement=false`: finish Specs landing (or `skip_specs_landing`) before apply
   - Implement: `llman-sdd-apply`
   - Validate: `llman sdd validate <id> --strict --no-interactive`
   - Review: `llman sdd change diff <id>` (read-only)
   - Close (recommended): `llman sdd change finalize <id>` (dirty tree OK; then one `git commit`)
   - Fallback: `llman sdd change checkpoint <id>` (clean tree required) → `llman sdd change archive <id>`

{{ unit("skills/git-native-flow") }}
{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

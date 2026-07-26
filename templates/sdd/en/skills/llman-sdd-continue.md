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
   - If `stage` is `draft` (proposal.md only), explicitly tell the user: "This is a draft proposal. Grow it to `full` (design → tasks → live specs → `change start`) before it can be implemented; a draft cannot be applied or verified directly." If proposal+design+tasks already exist but stage is still `draft`, the next step is `llman sdd change start <id>` (or `change attach`) on a non-default feature branch — do NOT create `changes/<id>/specs/`.
3. Determine the next artifact to create (in order):
   1) `proposal.md`
   2) live edits to `llmanspec/specs/<capability>/spec.toon` (+ `*.feature` when `bdd:` configured) on a feature branch
   3) `design.md` (only if design tradeoffs matter)
   4) `tasks.md`
   5) `llman sdd change start <id>` (or `change attach <id>` if the branch already exists)
4. Create exactly ONE missing artifact (or one live spec/feature edit on the branch).
   - Do NOT implement application code in continue mode.
   - Do NOT create `*.feature.delta.toon` or files under `changes/<id>/specs/`.
5. If all artifacts already exist, suggest next actions:
   - Implement: `llman-sdd-apply`
   - Validate: `llman sdd validate <id> --strict --no-interactive`
   - Review: `llman sdd change diff <id>` (read-only)
   - Close (recommended): `llman sdd change finalize <id>` (dirty tree OK; then one `git commit`)
   - Fallback: `llman sdd change checkpoint <id>` (clean tree required) → `llman sdd change archive <id>`

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

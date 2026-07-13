---
name: "llman-sdd-continue"
description: "Continue an existing llman SDD change by creating the next artifact."
metadata:
  version: "{{ llman_version }}"
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
   - If `stage` is `draft` (proposal.md only), explicitly tell the user: "This is a draft proposal. Grow it to `full` (specs → design → tasks) before it can be implemented; a draft cannot be applied or verified directly."
3. Determine the next artifact to create (in order):
   1) `proposal.md`
   2) `specs/<capability>/spec.toon` (one folder per capability)
   3) `design.md` (only if design tradeoffs matter)
   4) `tasks.md`
4. Create exactly ONE missing artifact under `llmanspec/changes/<id>/`.
   - Do NOT implement application code in continue mode.
5. If all artifacts already exist, suggest next actions:
   - Implement: `llman-sdd-apply`
   - Validate: `llman sdd validate <id> --strict --no-interactive`
   - Archive (when ready): `llman sdd archive <id>`

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

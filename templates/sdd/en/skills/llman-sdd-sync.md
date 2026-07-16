---
name: "llman-sdd-sync"
description: "Manually sync delta specs into main specs without archiving the change."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Sync

Use this skill to sync delta specs from an active change into main specs **without archiving** the change.

This is a manual, reproducible protocol.

## Steps
1. Select the change id (prompt the user if ambiguous).
   - Always announce: "Using change: <id>".
2. For each delta capability:
   - Constraints: `changes/<id>/specs/<capability>/spec.toon` → main `specs/<capability>/spec.toon`
   - Harness (if present): `*.feature.delta.toon` → main `*.feature` (or wait for `archive run` to apply)
   - Apply delta semantics manually; do **not** dual-write executable GWT into toon
3. Validate specs:
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```
4. Do NOT archive as part of sync. When ready, run `llman sdd archive run <id>`.

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

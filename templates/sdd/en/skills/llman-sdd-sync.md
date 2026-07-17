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
2. Mode check (`llmanspec/config.yaml`):
   - **BDD-on (Git-native)**: sync is unnecessary — live `llmanspec/specs/**` on the feature branch is SSOT. Use `llman sdd change diff <id>` for read-only review. Do **not** invent `feature_delta` apply. When ready: checkpoint → `change archive` (docs only) → Git/PR merge.
   - **BDD-off**: for each delta capability, apply `changes/<id>/specs/<capability>/spec.toon` → main `specs/<capability>/spec.toon` manually (classic TOON delta merge). No harness/branch requirements.
3. Validate specs:
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```
4. Do NOT archive as part of sync. When ready, run `llman sdd change archive <id>`.

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

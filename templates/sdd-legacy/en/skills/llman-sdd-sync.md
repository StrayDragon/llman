---
name: "llman-sdd-sync"
description: "Manually sync delta specs into main specs without archiving the change."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Sync

Use this skill to sync delta specs from an active change into main specs **without archiving** the change.

This is a manual, reproducible protocol.

## Steps
1. Select the change id (prompt the user if ambiguous).
2. For each delta spec at `llmanspec/changes/<id>/specs/<capability>/spec.md`:
   - Read the delta
   - Read (or create) the main spec: `llmanspec/specs/<capability>/spec.md`
   - Apply `ADDED|MODIFIED|REMOVED|RENAMED` sections manually
3. Validate specs:
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

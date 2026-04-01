---
name: "llman-sdd-sync"
description: "Manually sync delta specs into main specs without archiving the change."
metadata:
  llman-template-version: 3
---

# LLMAN SDD Sync

Use this skill to sync delta specs from an active change into main specs **without archiving** the change.

This is a manual, reproducible protocol.

## Steps
1. Select the change id (prompt the user if ambiguous).
   - Always announce: "Using change: <id>".
2. For each delta spec at `llmanspec/changes/<id>/specs/<capability>/spec.md`:
   - Read the delta
   - Read (or create) the main spec: `llmanspec/specs/<capability>/spec.md`
   - Apply the delta semantics manually (add/modify/remove/rename + scenarios), matching the project’s configured `spec_style` (`{{ spec_style }}`).
3. Validate specs:
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```
4. Do NOT archive as part of sync. When ready, run `llman sdd archive run <id>`.

{{ unit("skills/sdd-commands") }}
{{ unit_style("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

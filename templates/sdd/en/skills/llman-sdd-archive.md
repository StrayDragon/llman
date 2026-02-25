---
name: "llman-sdd-archive"
description: "Archive a change and merge deltas into specs."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Archive

Use this skill to archive a completed change.

## Steps
1. Ensure the change is deployed or accepted.
2. Run `llman sdd archive run <change-id>` (or legacy `llman sdd archive <change-id>`).
3. Use `--skip-specs` for tooling-only changes.
4. Use `--dry-run` to preview actions.
5. Re-run `llman sdd validate --strict --no-interactive`.

{{ unit("workflow/archive-freeze-guidance") }}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

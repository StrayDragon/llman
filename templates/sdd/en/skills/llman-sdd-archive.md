---
name: "llman-sdd-archive"
description: "Archive one or multiple changes and merge deltas into specs."
metadata:
  llman-template-version: 2
---

# LLMAN SDD Archive

Use this skill to archive completed changes.

## Steps
1. Confirm each target change is accepted or deployed.
2. Determine target IDs:
   - Single mode: one `<change-id>`.
   - Batch mode: multiple IDs (from user input or `llman sdd list --json`).
   - Always announce: "Archiving IDs: <id1>, <id2>, ...".
3. Validate each target first: `llman sdd validate <id> --strict --no-interactive`.
4. Optionally preview each archive: `llman sdd archive <id> --dry-run`.
5. Archive sequentially:
   - default: `llman sdd archive run <id>` (or `llman sdd archive <id>`)
   - tooling-only: `llman sdd archive run <id> --skip-specs`
   - stop immediately on first failure and report remaining IDs.
6. Run final validation once: `llman sdd validate --strict --no-interactive`.

{{ unit("workflow/archive-freeze-guidance") }}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

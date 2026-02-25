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
6. If archived directories are growing too large, maintain cold backup:
   - Preview freeze candidates: `llman sdd archive freeze --dry-run`
   - Freeze old archives: `llman sdd archive freeze --before <YYYY-MM-DD> --keep-recent <N>`
   - Restore when needed: `llman sdd archive thaw --change <YYYY-MM-DD-id>`

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}

{{region: templates/sdd/en/skills/shared.md#validation-hints}}

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}

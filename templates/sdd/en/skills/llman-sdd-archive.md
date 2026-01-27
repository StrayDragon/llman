---
llman-template-version: 1
name: "LLMAN SDD Archive"
description: "Archive a change and merge deltas into specs."
---

# LLMAN SDD Archive

Use this skill to archive a completed change.

## Steps
1. Ensure the change is deployed or accepted.
2. Run `llman sdd archive <change-id>`.
3. Use `--skip-specs` for tooling-only changes.
4. Use `--dry-run` to preview actions.
5. Re-run `llman sdd validate --strict --no-interactive`.

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}

{{region: templates/sdd/en/skills/shared.md#validation-hints}}

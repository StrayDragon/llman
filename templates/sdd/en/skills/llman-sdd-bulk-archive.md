---
name: "llman-sdd-bulk-archive"
description: "Batch archive multiple llman SDD changes safely."
metadata:
  llman-template-version: 1
---

# LLMAN SDD Bulk Archive

Use this skill to archive multiple changes (one-by-one) and then run a full validation.

## Steps
1. List active changes: `llman sdd list --json`.
2. Ask the user which change ids to archive (1+). Do not guess.
3. For each id, in order (stop on first failure):
   - (Recommended) validate: `llman sdd validate <id> --strict --no-interactive`
   - (Optional) preview: `llman sdd archive <id> --dry-run`
   - Archive: `llman sdd archive <id>` (or `--skip-specs` for tooling-only changes)
4. After all archives succeed, run:
   ```bash
   llman sdd validate --strict --no-interactive
   ```

{{region: templates/sdd/en/skills/shared.md#sdd-commands}}

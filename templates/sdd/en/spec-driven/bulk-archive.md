<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/opsx/bulk-archive.md (copied 2026-02-09; adapted for llman) -->

Archive multiple completed changes in llman SDD.

**Input**: Optionally specify change ids after `/opsx:bulk-archive` (space-separated). If omitted, prompt the user to select from active changes.

**Steps**

1. **Pick the change ids**

   Run:
   ```bash
   llman sdd list --json
   ```

   If no active changes exist, inform the user and STOP.

   Ask the user which change ids to archive (1+). Do not guess.

2. **Archive each change (stop on failure)**

   For each selected id, in order:

   - (Recommended) validate first:
     ```bash
     llman sdd validate <id> --strict --no-interactive
     ```
     If validation fails, STOP and ask the user whether to fix artifacts first or intentionally proceed.

   - (Optional) preview:
     ```bash
     llman sdd archive <id> --dry-run
     ```

   - Archive:
     ```bash
     llman sdd archive <id>
     ```

     For tooling-only changes, use:
     ```bash
     llman sdd archive <id> --skip-specs
     ```

   If any archive fails, STOP and report the error (do not continue).

3. **Final verification**

   Run:
   ```bash
   llman sdd validate --strict --no-interactive
   ```

**Output On Success**

```
## Bulk Archive Complete

Archived:
- <id-1>
- <id-2>
```

**Guardrails**
- Never archive without explicitly confirmed change ids
- Stop on the first failure and report it
- Prefer validating before archiving

<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/en/opsx/archive.md (copied 2026-02-09; adapted for llman) -->

Archive a completed change in llman SDD.

**Input**: Optionally specify a change id after `/opsx:archive` (e.g., `/opsx:archive add-auth`). If omitted, infer from context; if ambiguous, run `llman sdd list --json` and ask the user which change to archive.

**Steps**

1. **Select the change**

   - If an id is provided, use it.
   - Otherwise, run `llman sdd list --json` and prompt the user to pick the change id.

   **IMPORTANT**: Do NOT guess. Never archive without an explicitly confirmed id.

2. **(Recommended) Validate first**

   Run:
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```

   If validation fails, STOP and ask the user whether to fix the artifacts first or intentionally proceed.

3. **(Optional) Dry run**

   Preview what would happen:
   ```bash
   llman sdd archive <id> --dry-run
   ```

4. **Archive**

   Default behavior (recommended):
   ```bash
   llman sdd archive <id>
   ```

   Tooling-only change:
   ```bash
   llman sdd archive <id> --skip-specs
   ```

   Notes:
   - By default, this merges delta specs into `llmanspec/specs/` (if present) and moves the change to `llmanspec/changes/archive/YYYY-MM-DD-<id>/`.
   - If the archive target already exists, STOP and ask the user how they want to proceed.

5. **Verify**

   Run:
   ```bash
   llman sdd validate --strict --no-interactive
   ```

6. **(Optional) Freeze archived directories into cold backup**

   When `llmanspec/changes/archive/` accumulates many dated directories:
   ```bash
   llman sdd archive freeze --dry-run
   llman sdd archive freeze --before <YYYY-MM-DD> --keep-recent <N>
   ```

   Restore specific entries when needed:
   ```bash
   llman sdd archive thaw --change <YYYY-MM-DD-id>
   ```

**Output On Success**

```
## Archive Complete

**Change:** <id>
**Archived to:** llmanspec/changes/archive/YYYY-MM-DD-<id>/
```

**Guardrails**
- Never archive without an explicitly confirmed change id
- Stop on validation failure unless the user explicitly chooses to proceed
- Prefer `--dry-run` before archiving when uncertain
- Use freeze/thaw only for dated archive directories (`YYYY-MM-DD-*`) and keep a small recent window unfrozen when possible

{{region: templates/sdd/en/skills/shared.md#structured-protocol}}

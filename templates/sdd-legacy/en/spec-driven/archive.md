<!-- llman-template-version: 2 -->
<!-- source: llman sdd-legacy workflow templates/en/archive.md -->

Archive completed changes in llman SDD.

**Input**: Optionally provide one or more change ids after `/llman-sdd:archive` (space-separated). If omitted, infer from context; if ambiguous, run `llman sdd-legacy list --json` and ask the user to choose.

**Steps**

1. **Resolve target IDs**

   - If one or more ids are provided, use them in order.
   - Otherwise, run `llman sdd-legacy list --json` and ask the user to pick explicit IDs.

   **IMPORTANT**: Do NOT guess. Never archive without confirmed IDs.
   Always announce: "Archiving IDs: <id1>, <id2>, ...".

2. **(Recommended) Validate each target first**

   Run for each id:
   ```bash
   llman sdd-legacy validate <id> --strict --no-interactive
   ```

   If any validation fails, STOP and ask whether to fix artifacts first or intentionally proceed.

3. **(Optional) Dry run each archive**

   ```bash
   llman sdd-legacy archive <id> --dry-run
   ```

4. **Archive sequentially**

   Default behavior (recommended):
   ```bash
   llman sdd-legacy archive run <id>
   ```

   Tooling-only change:
   ```bash
   llman sdd-legacy archive run <id> --skip-specs
   ```

   Notes:
   - Archive targets are processed one by one.
   - Stop on first failure and report which IDs were completed vs pending.
   - Successful runs merge delta specs into `llmanspec/specs/` (if present) and move the change to `llmanspec/changes/archive/YYYY-MM-DD-<id>/`.

5. **Verify once after completion**

   ```bash
   llman sdd-legacy validate --strict --no-interactive
   ```

{{ unit("workflow/archive-freeze-guidance") }}

**Output On Success**

```
## Archive Complete

**Archived IDs:** <id1>, <id2>, ...
**Archive Root:** llmanspec/changes/archive/
```

**Guardrails**
- Never archive without explicitly confirmed IDs
- Stop on validation failure unless the user explicitly chooses to proceed
- Prefer `--dry-run` before archiving when uncertain
- For batch mode, stop on first failure and preserve an auditable completion list

{{ unit("skills/structured-protocol") }}

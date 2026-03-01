<!-- llman-template-version: 3 -->
<!-- source: OpenSpec templates/en/llman-sdd/sync.md (copied 2026-02-09) -->

Sync delta specs from an active change into main specs **without archiving** the change.

This is a manual, reproducible protocol: read delta specs under `llmanspec/changes/<id>/specs/` and apply them to `llmanspec/specs/`.

**Input**: Optionally specify a change id after `/llman-sdd:sync` (e.g., `/llman-sdd:sync add-auth`). If omitted, infer from context; if ambiguous, prompt the user to choose.

**Steps**

1. **Select the change**

   If an id is provided, use it. Otherwise run `llman sdd list --json` and ask the user which change to sync.
   Always announce: "Using change: <id>" and how to override (e.g., `/llman-sdd:sync <other>`).

2. **Find delta specs**

   Look for delta spec files at:
   - `llmanspec/changes/<id>/specs/<capability>/spec.md`

   If none exist, report it and STOP.

3. **Apply deltas to main specs**

   For each `<capability>` delta:
   - Read the delta spec.
   - Read (or create) the main spec:
     - `llmanspec/specs/<capability>/spec.md`

   Apply changes by canonical ISON ops (`table.ops` + `table.op_scenarios`):
   - `add_requirement`: append to `table.requirements` and append matching rows to `table.scenarios`
   - `modify_requirement`: update the existing `req_id` row and replace its scenario rows
   - `remove_requirement`: delete the `req_id` row and all its scenario rows
   - `rename_requirement`: update the requirement title only (keep `req_id` stable)

   If you create a new main spec file, include required YAML frontmatter (`llman_spec_valid_scope`, `llman_spec_valid_commands`, `llman_spec_evidence`) and author the spec body as canonical ISON (`object.spec` + `table.requirements` + `table.scenarios`).
   See the Canonical ISON Spec Contract in `llmanspec/AGENTS.md`.

4. **Validate**

   Run:
   - `llman sdd validate --specs --strict --no-interactive`

5. **Summarize**

   Summarize which specs changed and what was added/modified/removed/renamed.

**Guardrails**
- Preserve existing content not mentioned in the delta unless the user asks otherwise
- If anything is unclear, ask before editing main specs

{{ unit("skills/structured-protocol") }}

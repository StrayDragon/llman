## 1. Dependencies and Core Types

- [ ] 1.1 Add `ison-rs` (and only necessary features) to `Cargo.toml`.
- [ ] 1.2 Introduce a dedicated module for table/object ISON parsing and dumping for llmanspec specs/deltas.
- [ ] 1.3 Define internal adapters that map canonical blocks (`object.spec`, `table.requirements`, `table.scenarios`, `object.delta`, `table.ops`, `table.op_scenarios`) into existing Rust domain structs used by `show/list/validate/archive`.

## 2. Legacy Command Split (`llman sdd-legacy`)

- [ ] 2.1 Add a top-level CLI command `llman sdd-legacy ...` that preserves the current JSON-in-` ```ison ` parsing/validation behavior.
- [ ] 2.2 Keep legacy JSON parsing + JSON repair logic isolated to `sdd-legacy` so `llman sdd` can be table/object ISON-only.
- [ ] 2.3 Ensure `llman sdd` and `llman sdd-legacy` have clear, non-overlapping error messages and legacy-command hints.
- [ ] 2.4 Ensure template defaults align with the command identity:
  - `llman sdd` uses the new templates
  - `llman sdd-legacy` uses the legacy templates

## 3. ISON Extraction, Multi-Block Support, and Merge Rules

- [ ] 3.1 Extend code-fence extraction to collect **all** fenced ` ```ison ` blocks from a file (not only the first).
- [ ] 3.2 Implement deterministic merge-by-block-name logic for multi-block files; treat duplicate canonical block names as validation errors.
- [ ] 3.3 Preserve “Markdown + ISON mixed” authoring: ensure parsing never depends on Markdown heading hierarchy, only on fenced ISON blocks.

## 4. Main Spec Parsing and Validation (Table/Object ISON)

- [ ] 4.1 Implement table/object ISON parsing for main specs with strict fixed block names and fixed columns (including `table.scenarios` with `given/when/then` columns).
- [ ] 4.2 Enforce canonical validation rules: `kind`, `name` (strict), uniqueness constraints, and ≥1 scenario per requirement.
- [ ] 4.3 Make `llman sdd` fail fast on legacy JSON-in-` ```ison ` payloads (not only `--strict`) with an actionable hint to use `llman sdd-legacy ...` (and to manually rewrite payloads into canonical table/object ISON when ready).
- [ ] 4.4 Ensure output JSON shape for `llman sdd show --json` remains stable (only on-disk format changes).

## 5. Delta Spec Parsing and Validation (Ops Tables)

- [ ] 5.1 Implement table/object ISON parsing for delta specs using `object.delta`, `table.ops`, `table.op_scenarios` (including `given/when/then` columns for op scenarios).
- [ ] 5.2 Enforce op/field rules (including `~` null handling and strict scenario rules for add/modify vs remove/rename).
- [ ] 5.3 Make `llman sdd` fail fast on legacy delta JSON payloads (not only `--strict`) with actionable legacy-command hints.

## 6. Deterministic Dumps and Write Paths

- [ ] 6.1 Implement token-friendly default dumping (no alignment padding) with stable block ordering.
- [ ] 6.2 Add an opt-in pretty alignment mode/flag for review (must not become the default).
- [ ] 6.3 Ensure archive merge writes canonical table/object ISON payloads and preserves deterministic ordering.

## 7. Authoring Helpers (integrated into `llman sdd`)

- [ ] 7.1 Add `llman sdd spec skeleton <capability>` to generate a valid main spec skeleton (including required frontmatter placeholders).
- [ ] 7.2 Add `llman sdd delta skeleton <change-id> <capability>` to generate a valid delta spec skeleton.
- [ ] 7.3 Add `llman sdd delta add-op ...` to append an op row with correct null placeholders.
- [ ] 7.4 Add `llman sdd delta add-scenario ...` to append an op scenario row (`given/when/then`) and validate linkage to add/modify ops.
- [ ] 7.5 Ensure all authoring helpers emit token-friendly dumps by default and support opt-in pretty alignment.
- [ ] 7.6 Add `llman sdd show <spec> --type spec --json --meta-only` to return lightweight metadata for agents (feature name/purpose) without returning the full `requirements` array.
- [ ] 7.7 Add an opt-in `--compact-json` mode for JSON outputs used by agents (token-friendly) while keeping the default pretty JSON output review-friendly.

## 8. Template and Prompt Updates (Agent Guidance)

- [ ] 8.1 Update `templates/sdd/**` guidance that currently references Markdown heading deltas (`## ADDED|MODIFIED|...`) to instead teach canonical table/object ISON blocks and columns.
- [ ] 8.2 Add a globally injected “ISON spec contract” section to llmanspec-managed instructions (for example in `llmanspec/AGENTS.md`) so templates can reference it rather than repeating long schema text.
- [ ] 8.3 Update shared validation hints units to show ISON table examples and common fixes; forbid invalid pseudo-markers inside fenced ISON (for example, `<meta-directives>`).
- [ ] 8.4 Run `just check-sdd-templates` to ensure locale parity and template version checks pass.

## 9. Tests and Acceptance

- [ ] 9.1 Update integration tests that write JSON payloads in ` ```ison ` blocks to table/object ISON examples (for the new `llman sdd` path).
- [ ] 9.2 Add tests for:
  - multi-block ` ```ison ` merge behavior
  - `llman sdd` rejection of legacy JSON payloads with correct hint text
  - `llman sdd-legacy` acceptance of legacy JSON payloads
  - deterministic dumps (same output across runs)
  - authoring helper commands producing strict-valid files
- [ ] 9.3 Run `just test` and `just check` (clippy with `-D warnings`) to validate end-to-end.

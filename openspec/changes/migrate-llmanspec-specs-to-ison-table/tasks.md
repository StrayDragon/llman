## 1. Dependencies and Core Types

- [x] 1.1 Add `ison-rs` (and only necessary features) to `Cargo.toml`.
- [x] 1.2 Introduce a dedicated module for table/object ISON parsing and dumping for llmanspec specs/deltas.
- [x] 1.3 Define internal adapters that map canonical blocks (`object.spec`, `table.requirements`, `table.scenarios`, `object.delta`, `table.ops`, `table.op_scenarios`) into existing Rust domain structs used by `show/list/validate/archive`.

## 2. Legacy Command Split (`llman sdd-legacy`)

- [x] 2.1 Add a top-level CLI command `llman sdd-legacy ...` that preserves the current JSON-in-` ```ison ` parsing/validation behavior.
- [x] 2.2 Keep legacy JSON parsing + JSON repair logic isolated to the `sdd-legacy` command so `llman sdd` can be table/object ISON-only.
- [x] 2.3 Ensure `llman sdd` and `llman sdd-legacy` have clear, non-overlapping error messages and legacy-command hints.
- [x] 2.4 Ensure template defaults align with the command identity:
  - `llman sdd` uses the new templates
  - `llman sdd-legacy` uses the legacy templates

## 3. ISON Extraction, Multi-Block Support, and Merge Rules

- [x] 3.1 Extend code-fence extraction to collect **all** fenced ` ```ison ` blocks from a file (not only the first).
- [x] 3.2 Implement deterministic merge-by-block-name logic for multi-block files; treat duplicate canonical block names as validation errors.
- [x] 3.3 Preserve “Markdown + ISON mixed” authoring: ensure parsing never depends on Markdown heading hierarchy, only on fenced ISON blocks.

## 4. Main Spec Parsing and Validation (Table/Object ISON)

- [x] 4.1 Implement table/object ISON parsing for main specs with strict fixed block names and fixed columns (including `table.scenarios` with `given/when/then` columns).
- [x] 4.2 Enforce canonical validation rules: `kind`, `name` (strict), uniqueness constraints, and ≥1 scenario per requirement.
- [x] 4.3 Make `llman sdd` fail fast on legacy JSON-in-` ```ison ` payloads (not only `--strict`) with an actionable hint to use `llman sdd-legacy ...` (and to manually rewrite payloads into canonical table/object ISON when ready).
  - Legacy JSON detection must be a fast sniff: after trimming leading whitespace, `{` or `[` means legacy JSON (emit a dedicated error message, not an `ison-rs` parse error).
- [x] 4.4 Ensure output JSON shape for `llman sdd show --json` remains stable (only on-disk format changes).
  - Ensure `given/when/then` rows map deterministically to `Scenario.rawText` (compat output).

## 5. Delta Spec Parsing and Validation (Ops Tables)

- [x] 5.1 Implement table/object ISON parsing for delta specs using `object.delta`, `table.ops`, `table.op_scenarios` (including `given/when/then` columns for op scenarios).
- [x] 5.2 Enforce delta validation rules: `kind`, op/field rules (including `~` null handling and strict scenario rules for add/modify vs remove/rename).
- [x] 5.3 Make `llman sdd` fail fast on legacy delta JSON payloads (not only `--strict`) with actionable legacy-command hints.
  - Use the same legacy JSON sniff rule as main specs.

## 6. Deterministic Dumps and Write Paths

- [x] 6.1 Implement token-friendly default dumping (no alignment padding) with stable canonical block ordering (`object.*` → main table → scenarios table).
- [ ] 6.2 Add an opt-in pretty alignment mode/flag for review (must not become the default; maps directly to `ison-rs` dumper alignment).
- [x] 6.3 Ensure row ordering is stable (no auto-sorting); writers preserve existing order and append new rows at the end.
- [x] 6.4 Ensure archive merge writes canonical table/object ISON payloads and preserves deterministic ordering.

## 7. Authoring Helpers (integrated into `llman sdd`)

- [ ] 7.1 Add `llman sdd spec skeleton <capability>` to generate a valid main spec skeleton (including optional frontmatter placeholders).
- [ ] 7.2 Add `llman sdd delta skeleton <change-id> <capability>` to generate a valid delta spec skeleton (MUST omit YAML frontmatter).
- [ ] 7.3 Add `llman sdd spec add-requirement ...` to append a requirement row (`req_id`, `title`, `statement`) with deterministic ordering and strict validation.
- [ ] 7.4 Add `llman sdd spec add-scenario ...` to append a scenario row (`req_id`, `id`, `given`, `when`, `then`) and validate linkage to an existing requirement.
- [ ] 7.5 Add `llman sdd delta add-op ...` to append an op row with correct null placeholders.
- [ ] 7.6 Add `llman sdd delta add-scenario ...` to append an op scenario row (`given/when/then`) and validate linkage to add/modify ops.
- [ ] 7.7 Ensure all authoring helpers emit token-friendly dumps by default and support opt-in pretty alignment.
- [x] 7.8 Add `llman sdd show <spec> --type spec --json --meta-only` to return lightweight metadata for agents (feature name/purpose) without returning the full `requirements` array.
- [x] 7.9 Add an opt-in `--compact-json` mode for `llman sdd list/show/validate --json` (and `llman sdd-legacy ...`) to emit JSON without pretty whitespace (token-friendly) while keeping the default pretty JSON output review-friendly.

## 8. Template and Prompt Updates (Agent Guidance)

- [x] 8.1 Update `templates/sdd/**` guidance that currently references Markdown heading deltas (`## ADDED|MODIFIED|...`) to instead teach canonical table/object ISON blocks and columns.
- [x] 8.2 Add a globally injected “ISON spec contract” section to llmanspec-managed instructions (for example in `llmanspec/AGENTS.md`) so templates can reference it rather than repeating long schema text.
- [x] 8.3 Update shared validation hints units to show ISON table examples and common fixes; forbid invalid pseudo-markers inside fenced ISON (for example, `<meta-directives>`).
- [x] 8.4 Run `just check-sdd-templates` to ensure locale parity and template version checks pass.

## 9. Tests and Acceptance

- [x] 9.1 Update integration tests that write JSON payloads in ` ```ison ` blocks to table/object ISON examples (for the new `llman sdd` path).
- [ ] 9.2 Add tests for:
  - multi-block ` ```ison ` merge behavior
  - `llman sdd` rejection of legacy JSON payloads with correct hint text (dedicated sniff-based error)
  - `llman sdd-legacy` acceptance of legacy JSON payloads
  - `given==""` vs `given!=""` mapping to `Scenario.rawText` is deterministic and stable
  - pretty alignment flag changes whitespace only (semantic round-trip stays the same)
  - deterministic dumps (same output across runs)
  - authoring helper commands producing strict-valid files
- [x] 9.3 Run `just test` and `just check` (clippy with `-D warnings`) to validate end-to-end.

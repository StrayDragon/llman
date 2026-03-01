## Why

Today `llmanspec/**/spec.md` uses a fenced ` ```ison ` block, but the payload is actually **JSON** (with a repair fallback). This mismatch with “real ISON” (table/object syntax) causes confusion for authors and agents, increases token cost, and makes safe incremental edits (CRUD on requirements/scenarios) unnecessarily manual and error-prone. In practice it also creates drift: parts of `templates/sdd/**` still guide authors toward Markdown heading conventions that are no longer the runtime semantic source.

We want a single, canonical, human-editable format that matches the ISON ecosystem, is token-efficient, and is easy to modify mechanically with stable formatting. For large changes, we also need first-class CLI helpers to generate spec skeletons and apply delta ops/scenario edits safely instead of hand-editing long payloads.

## What Changes

- Switch the canonical payload inside ` ```ison ` blocks for:
  - `llmanspec/specs/<capability>/spec.md` (main specs), and
  - `llmanspec/changes/<change-id>/specs/<capability>/spec.md` (delta specs)
  from **JSON** to **table/object ISON** (parsed/dumped via a Rust ISON crate).
- Update SDD runtime parsing/validation/archive-merge to treat table/object ISON as the primary semantic source.
- Introduce an explicit legacy workflow command to preserve the current JSON-in-` ```ison ` parsing behavior:
  - `llman sdd-legacy` maintains the previous parsing/validation behavior for legacy repositories.
  - `llman sdd` becomes table/object ISON-first and fails fast on legacy JSON payloads with a concrete legacy-command hint and rewrite guidance.
- Extend the SDD CLI with ISON authoring helpers:
  - generate/update main spec and delta spec skeletons,
  - apply main spec CRUD (add requirement + add scenario),
  - apply delta ops CRUD (add/modify/remove/rename requirement),
  - append/edit scenarios at the `req_id` + `scenario.id` level (scenarios are structured rows with `given/when/then` columns),
  - provide lightweight spec metadata output for agents (fetch feature name/purpose without retrieving full requirement bodies),
  - emit deterministic, token-friendly dumps by default with an opt-in pretty alignment mode for review.
- Update new-style SDD templates/units so agents are instructed to author specs/deltas using the new ISON table schema (and to prefer the CLI helpers), replacing outdated guidance that references Markdown heading sections like `## ADDED|MODIFIED|...`.

Non-goals (explicit):
- Do not deduplicate `templates/sdd/**` vs `templates/sdd-legacy/**` yet (we will keep parity and test stability first).
- Do not migrate `openspec/specs/**` canonical docs to ISON (scope is `llmanspec` only).
- Do not require Python tooling at runtime (pure Rust implementation).

## Capabilities

### New Capabilities

- `sdd-ison-authoring`: Define the canonical table/object ISON schema for llmanspec spec/delta payloads and the CLI authoring/editing commands (skeleton generation + spec/delta CRUD), including stable dump ordering and token-friendly default formatting.

### Modified Capabilities

- `sdd-ison-pipeline`: Update the “ISON container” contract to specify table/object ISON as the primary payload format (not JSON), and define compatibility rules via an explicit `llman sdd-legacy` command instead of implicit runtime tolerance.
- `sdd-workflow`: Extend SDD CLI/workflow guidance to include the new authoring commands and update the expected spec/delta authoring flow (creating/editing `llmanspec/**/spec.md` via the table ISON schema).
- `sdd-structured-skill-prompts`: Update template guidance and validation hints to reflect the new ISON table schema (examples, common fixes, and avoiding non-ISON markers inside ` ```ison ` blocks).

## Impact

- **Spec engine**: update parsing/validation/merge paths currently built around JSON payloads:
  - `src/sdd/spec/ison.rs`, `src/sdd/spec/parser.rs`, `src/sdd/spec/validation.rs`
  - `src/sdd/change/delta.rs`, `src/sdd/change/archive.rs`
- **CLI surface**: add a new subcommand group under `llman sdd` for ISON authoring/editing (skeleton + spec/delta CRUD), and keep existing commands (`show/list/validate/archive`) behavior stable at the JSON output level.
- **Templates**: update `templates/sdd/**` (and shared units like validation hints) to instruct table ISON authoring and reference the new CLI helpers; keep locale parity and template-version checks passing.
- **Tests**: update integration tests that currently write JSON payloads in ` ```ison ` blocks; add coverage for:
  - legacy vs new command behavior on old payloads (fail fast with actionable hints),
  - stable dumps/ordering and strict validation diagnostics.
- **User-visible changes**:
  - `llmanspec/**/spec.md` content becomes easier to review/diff and cheaper to include in prompts,
  - strict validation errors will reference missing blocks/columns/rows rather than Markdown headings,
  - users can stay on legacy JSON payloads via `llman sdd-legacy` and manually choose when to rewrite to table/object ISON.

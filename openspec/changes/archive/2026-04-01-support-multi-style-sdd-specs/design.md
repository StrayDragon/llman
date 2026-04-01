## Context

`llman sdd` currently assumes a single on-disk spec style:

- `llmanspec/config.yaml` only carries `version`, `locale`, and skills paths.
- `src/sdd/spec/parser.rs`, `src/sdd/spec/validation.rs`, and `src/sdd/change/archive.rs` only understand canonical table/object ISON payloads.
- `src/sdd/authoring/spec.rs` and `src/sdd/authoring/delta.rs` always emit ISON.
- `src/sdd/command.rs` exposes import/export and authoring helpers, but there is no first-class command for converting one SDD style into another.

The proposal changes the contract from “single canonical ISON payload” to “single project-wide primary style chosen explicitly from `ison`, `toon`, or `yaml`”. This must stay strict: no implicit default on read paths, no mixed-style repository tolerance, and no best-effort auto-detection fallback once a project declares its style.

At the same time, the semantic layer must remain stable. `show`, `list`, `validate`, `archive`, and authoring helpers should keep operating on the same requirement/scenario/op model regardless of whether the file payload is encoded as ISON, TOON, or YAML.

## Goals / Non-Goals

**Goals:**
- Add an explicit `spec_style` setting to `llmanspec/config.yaml`, with supported values `ison`, `toon`, and `yaml`.
- Keep `llmanspec/specs/**/spec.md` and `llmanspec/changes/**/specs/**/spec.md` as the stable file locations while allowing the canonical fenced payload inside them to vary by style.
- Parse all three styles into one shared semantic model before validation, JSON rendering, archive merge, and write-back.
- Make all SDD read/write commands fail loudly when style configuration is missing or when a file does not match the configured style.
- Provide explicit conversion workflows for both whole-project migrations and single-file review/migration steps.
- Update authoring helpers, templates, and generated skills so new output matches the project’s configured primary style.

**Non-Goals:**
- Do not allow mixed styles inside one active project as a steady state.
- Do not add a compatibility mode that guesses the file style from content when `spec_style` is missing.
- Do not change the external JSON shape returned by `llman sdd show --json`, `list --json`, or `validate --json`.
- Do not change OpenSpec import/export semantics in this change.

## Decisions

### 1) Require explicit project style in `llmanspec/config.yaml`

Add a `SpecStyle` enum to `src/sdd/project/config.rs` and expose it in `llmanspec-config.schema.json` as a required enum-backed field:

- `spec_style: ison | toon | yaml`

Behavior:
- `llman sdd init` MUST write `spec_style: ison` explicitly for new projects.
- Commands that only manage templates/prompts (`init`, `update`, `update-skills`) may still create or refresh config defaults.
- Commands that read or mutate spec/delta payloads (`list --specs`, `show`, `validate`, `archive`, `spec`, `delta`, and the new `convert`) MUST require an existing config with an explicit `spec_style`.
- Missing/unknown `spec_style` becomes a hard error with a concrete hint to edit `llmanspec/config.yaml`.

This keeps new projects usable while preventing existing repos from silently inheriting an implicit default during spec operations.

### 2) Keep Markdown files, switch canonical fenced payload by style

The file layout stays the same:

- `llmanspec/specs/<capability>/spec.md`
- `llmanspec/changes/<change>/specs/<capability>/spec.md`

Each file continues to allow surrounding Markdown prose, but the canonical payload fence now depends on `spec_style`:

- `ison`: one or more fenced ` ```ison ` blocks using the current canonical block contract.
- `toon`: one fenced ` ```toon ` block containing one TOON document.
- `yaml`: one fenced ` ```yaml ` block containing one YAML document.

Main-spec semantic keys stay stable across styles:

- `kind`
- `name`
- `purpose`
- `requirements[]` with `req_id`, `title`, `statement`
- `scenarios[]` with `req_id`, `id`, `given`, `when`, `then`

Delta-spec semantic keys stay stable across styles:

- `kind`
- `ops[]` with `op`, `req_id`, `title`, `statement`, `from`, `to`, `name`
- `op_scenarios[]` with `req_id`, `id`, `given`, `when`, `then`

ISON keeps its multi-block table/object form because it is already optimized for incremental row-level edits. TOON and YAML use a single object document because that maps naturally onto their native serializers and avoids inventing style-specific block-merging rules where none are needed.

### 3) Introduce a shared semantic IR plus style backends

Refactor the SDD parsing/writing pipeline around one semantic intermediate representation:

- main spec IR
- delta spec IR
- shared validation helpers for requirement/scenario/op invariants

Proposed module split:
- `src/sdd/project/config.rs`: `SpecStyle`
- `src/sdd/spec/mod.rs`: style backend trait and IR types
- `src/sdd/spec/parser.rs`: config-aware dispatch into style backends
- `src/sdd/spec/validation.rs`: style-agnostic semantic validation plus style-specific envelope checks
- new backend modules under `src/sdd/spec/` for `ison`, `toon`, and `yaml`

Backend responsibilities:
- validate the expected fence kind for the configured style
- parse payload into IR
- serialize IR back into deterministic style-specific text
- report precise style mismatch errors (`expected yaml fence, found ison fence`, etc.)

Command-facing code (`show`, `list`, `validate`, `archive`, authoring helpers) should only consume IR, not raw style-specific structures.

### 4) Choose concrete backend libraries per style (no alternates in-scope)

The backend trait should not stay abstract for long. This change standardizes on one fixed library path per style so implementation can proceed without re-litigating dependencies.

- `ison`
  - keep `ison-rs` as the parsing backend because the repository already depends on it and already contains a targeted workaround for a real parser edge case in the current SDD path.
  - wrap `ison-rs` behind an adapter module so all parser quirks stay inside the `ison` backend instead of leaking into command code or IR validation.
  - do not replace the crate during this change unless implementation proves it blocks semantic parity.

- `toon`
  - use `serde_toon_format` as the only TOON backend.
  - rationale: serde-native API, strict validation hooks, reader/writer helpers, and a conformance-oriented test story make it the best fit for llman's existing Rust pipeline.
  - other TOON crates are explicitly out-of-scope for this change.

- `yaml`
  - use `serde_yaml` for semantic parsing and deterministic fresh serialization (conversion + file creation).
  - use `yamlpatch` as the lossless write-back engine for in-place updates where preserving comments/formatting matters.
  - do not introduce `yaml-edit` or `fyaml` in this change.

This keeps the first implementation decisive. If a different backend is needed later, it should be handled as a separate change with explicit evaluation.

### 5) Split YAML into semantic overlay planning and lossless write-back

YAML needs one more layer than ISON and TOON because the project explicitly wants `ruamel.yaml`-like comment retention where feasible.

The YAML backend should therefore be split into two responsibilities:

- semantic parse / normalize
  - parse the fenced YAML payload into the same IR used by the other styles.
  - enforce the same required keys and identifier constraints as ISON/TOON.
  - keep deterministic serializer support for fresh file generation and cross-style conversion output.

- lossless update / overlay
  - when mutating an existing YAML-backed spec in place, do not rewrite the whole document unless there is no safer option.
  - compute a recursive overlay plan from the old IR to the new IR.
  - apply that plan against the original YAML source using comment-preserving patch operations.

This separation avoids coupling semantic correctness to a single editing crate and makes testing easier:

- IR tests verify semantic parity across styles.
- YAML patch tests verify preservation of comments, whitespace, indentation, and stable ordering.

### 6) Use an identifier-aware overlay planner for YAML, not a generic deep merge

Generic deep merge is not enough for SDD because several collections have semantic identities:

- main spec requirements are keyed by `req_id`
- main spec scenarios are keyed by `(req_id, id)`
- delta spec ops and op scenarios are keyed by their semantic row identity rather than raw array index intent

The YAML write path should therefore use an SDD-aware overlay planner with these rules:

- mappings recurse by key
- scalar or type-changing updates replace the old value
- semantically keyed collections are matched by their identifiers before deciding whether an entry is updated, inserted, removed, or reordered
- serializer output for newly created entries must remain deterministic

For the first implementation, this planner may keep array behavior intentionally strict rather than fully generic. A narrow, SDD-specific merge is preferable to a broad but lossy merge that destroys comments or reflows unrelated sections.

Concrete overlay rules for YAML in this change:

- Patch operations: generate only `Replace`, `Add`, `Remove`, and `Append` operations. Do not rely on `yamlpatch::MergeInto` for semantic merge behavior.
- Ordering: preserve existing list order for all semantically keyed collections; add new entries at the end of the list; remove entries in place. Do not auto-reorder existing entries as part of overlay.
- Scope of preservation: preserve comments/formatting for untouched YAML regions. A value that is directly replaced may lose inline comments attached to that exact value; this is acceptable as long as unrelated regions remain unchanged.
- Fallback when lossless overlay fails: regenerate the fenced YAML payload deterministically from IR and replace only the fenced payload block (leave surrounding Markdown untouched). This loses YAML comments inside the payload, but keeps semantics correct.

This is the explicit "ruamel-like if possible, else degrade to conventional" policy for YAML within this change.

Deterministic fresh serialization rules (used for `convert` output, new file creation, and YAML overlay fallback regeneration):

- Canonical key order (main spec): `kind`, `name`, `purpose`, `requirements`, `scenarios`.
- Canonical key order (delta spec): `kind`, `ops`, `op_scenarios`.
- Canonical key order (requirement entry): `req_id`, `title`, `statement`.
- Canonical key order (scenario entry): `req_id`, `id`, `given`, `when`, `then`.
- Canonical key order (delta op entry): `op`, `req_id`, `title`, `statement`, `from`, `to`, `name`.
- Canonical key order (delta op scenario entry): `req_id`, `id`, `given`, `when`, `then`.
- Indentation: use 2 spaces for nested structures, and always end the fenced payload with a trailing newline.
- YAML emission: emit block-style mappings and sequences (no flow style). Anchor/alias emission is not used.
- TOON emission: use strict, canonical encoding; do not enable key folding or path expansion by default.

### 7) Add explicit conversion as a first-class SDD workflow

Introduce `llman sdd convert` with two safe scopes:

- project migration:
  - `llman sdd convert --to <style> --project`
  - converts all main specs and active change delta specs in place
  - validates every source file before any write
  - reparses converted output before commit
  - updates `llmanspec/config.yaml` only after all targeted files are written successfully

- single-file conversion:
  - `llman sdd convert --to <style> --file <path> [--output <path>]`
  - converts one main spec or delta spec
  - if `--output` is omitted, prints the converted document to stdout
  - does not rewrite project config

Both modes should support `--dry-run` summaries so the user can audit the conversion plan before writes. This gives maintainers a safe whole-project migration path while still allowing one-file review or staged migration work.

### 8) Make authoring helpers and archive write-back style-aware

Existing authoring commands stay in place:

- `llman sdd spec ...`
- `llman sdd delta ...`

But their output must follow the configured style:

- in `ison` projects, keep current table/object ISON write paths
- in `toon` projects, emit canonical TOON documents
- in `yaml` projects, emit canonical YAML documents

Formatting rules:
- ISON keeps token-friendly default dumps plus optional `--pretty-ison`
- TOON/YAML must emit stable key ordering and deterministic list ordering
- `--pretty-ison` should error outside `ison` projects instead of being silently ignored

`src/sdd/change/archive.rs` should stop assuming ISON write-back. Archive merge must operate on IR and serialize the updated main spec in the project’s configured style.

For YAML projects, archive write-back should prefer the lossless overlay path over full-document regeneration when updating an existing main spec. Full regeneration is acceptable for conversions into YAML or for first-time file creation, but not as the default update strategy for an already human-edited YAML spec.

### 9) Update templates, generated skills, and help text to surface the configured style

The workflow guidance rendered by `llman sdd update` and `llman sdd update-skills` must stop hard-coding ISON examples. Instead:

- generated instructions should mention the current `spec_style`
- examples for spec/delta authoring must match that style
- help/errors for `convert` and style config must label `toon` and `yaml` as experimental

This prevents agents from generating the wrong fenced payload format after a project has switched away from ISON.

## Risks / Trade-offs

- [Risk] `serde_toon_format` may still expose edge cases once real SDD examples hit it.
  - Mitigation: isolate TOON behind a backend trait and add round-trip fixtures for main spec and delta spec shapes.
- [Risk] Hard style gating will break repositories that relied on implicit defaults or partial manual conversions.
  - Mitigation: `init` writes `spec_style: ison` for new projects, `convert --project` handles whole-repo migration, and errors must name the exact file plus expected style.
- [Risk] Project conversion can leave a repo inconsistent if config is rewritten before every file succeeds.
  - Mitigation: validate all sources up front, write converted files first, reparse them, and update config last.
- [Risk] YAML comment-preserving updates are materially more complex than deterministic regeneration.
  - Mitigation: separate semantic IR from write-back, keep the overlay planner SDD-specific, and test preservation on representative commented fixtures before broadening scope.
- [Risk] `yamlpatch` may not cover every nested structure or collection rewrite needed by archive merge.
  - Mitigation: use lossless overlay first and fall back to deterministic regeneration of the fenced YAML payload when patch application fails.
- [Risk] Style-specific examples in templates/skills can drift from runtime behavior.
  - Mitigation: render examples from the same backend-specific serializers used by authoring helpers where practical, or at minimum keep shared fixture tests that validate generated examples.

## Migration Plan

1. Extend `llmanspec/config.yaml` and schema generation with `spec_style`, and make `init` emit `spec_style: ison`.
2. Introduce style-aware config loading for all spec-reading/spec-writing command paths.
3. Refactor parser/validator/archive/authoring to use a shared IR with `ison`/`toon`/`yaml` backends.
4. Implement the concrete backend adapters:
   - `ison-rs` adapter with current workaround encapsulation
   - `serde_toon_format` adapter
   - YAML parser plus lossless write-back path
5. Add the YAML identifier-aware overlay planner and preservation-focused fixture coverage.
6. Add `llman sdd convert` for whole-project and single-file conversion.
7. Update templates, generated skills, and error/help text to reference the configured style and experimental boundaries.
8. Add integration coverage for missing config, style mismatch, three-style parse/validate/show behavior, archive merge, conversion rollback/success paths, and YAML comment preservation.

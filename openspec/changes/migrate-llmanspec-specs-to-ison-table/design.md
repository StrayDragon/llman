## Context

`llmanspec/specs/**/spec.md` and `llmanspec/changes/**/specs/**/spec.md` currently embed a fenced ` ```ison ` code block whose payload is **JSON**, not ISON table/object syntax. The runtime parser (`src/sdd/spec/ison.rs`) treats this payload as JSON and even includes a JSON “repair” fallback. This is confusing for humans and agents (“ISON” is not ISON), and it makes safe, incremental edits (CRUD on requirements/scenarios/ops) hard to automate without writing JSON patch logic.

At the same time, the rest of the system increasingly treats the fenced payload as the canonical semantic source (show/list/validate/archive merge). We want that canonical semantic source to be the actual ISON format the team wants to standardize on, parsed and written in pure Rust.

Constraints:
- Scope is `llmanspec/**` only (not `openspec/specs/**`).
- Runtime must remain pure Rust (no Python tooling dependency).
- Must keep a transition path for existing repositories via an explicit legacy command (`llman sdd-legacy`).
- Output APIs (`llman sdd show --json`, etc.) should remain stable (payload format changes are file-format, not output-format).

## Goals / Non-Goals

**Goals:**
- Make **table/object ISON** the canonical payload inside ` ```ison ` blocks for:
  - main specs: `llmanspec/specs/<capability>/spec.md`
  - delta specs: `llmanspec/changes/<change-id>/specs/<capability>/spec.md`
- Use a Rust crate (`ison-rs`) to parse + dump ISON with stable ordering and predictable diffs.
- Add first-class CLI authoring helpers (skeleton generation + delta ops/scenario CRUD) so users/agents avoid hand-editing large payloads.
- Preserve legacy behavior via an explicit legacy command (`llman sdd-legacy`) so existing repositories remain usable without loosening the new format contracts.
- Allow users to manually rewrite legacy payloads into canonical table/object ISON when they choose to switch from `llman sdd-legacy` to `llman sdd`.
- Update `templates/sdd/**` guidance so agents author new specs/deltas in the new ISON schema (and prefer the new CLI helpers).

**Non-Goals:**
- Do not deduplicate `templates/sdd/**` vs `templates/sdd-legacy/**` in this change.
- Do not require Python at runtime for parsing or validation.
- Do not change the *external* command JSON output schema (only the on-disk spec/delta file format).
- Do not migrate `openspec/specs/**` canonical docs.

## Decisions

### 1) Adopt `ison-rs` for parsing and dumping (pure Rust)
- Use `ison-rs` (v1.0.x) to parse and dump the payload inside ` ```ison ` blocks.
- Rationale:
  - pure Rust runtime (no external toolchain dependency),
  - already supports tables/objects/references/types and stable dumps,
  - enables deterministic rewrites for CRUD commands.

Alternatives considered:
- Call Python `ison-py`/`isonantic` from Rust: faster to prototype but adds runtime dependency and cross-platform complexity.
- Write a bespoke parser: highest control but unnecessary effort given available crates.

### 2) Keep the ` ```ison ` fence as the container boundary
- Continue to use ` ```ison ` fenced code blocks as the extraction boundary in Markdown files.
- Files MAY include Markdown headings/prose mixed with ISON blocks (for readability).
- Runtime semantics MUST come from parsing the ` ```ison ` blocks, not from Markdown heading structure.
- The payload format changes (JSON → ISON table/object).
- Rationale: preserve the existing “ISON container” boundary while allowing authors to keep helpful Markdown structure around the ISON blocks.

### 3) Canonical block schema for main specs
Inside the ` ```ison ` payload for `llmanspec/specs/<capability>/spec.md`, define canonical blocks:

- `object.spec` (exactly 1 row; fields: `version kind name purpose`)
- `table.requirements` (fields: `req_id title statement`)
- `table.scenarios` (fields: `req_id id given when then`)

Block names MUST be strictly fixed to the identifiers above (no aliases), because they are part of the normative authoring/CRUD contract.

Validation contract:
- `kind` MUST be `llman.sdd.spec`
- `name` MUST match `<capability>` in strict mode
- each requirement MUST have ≥1 scenario row
- `(req_id, id)` MUST be unique in scenarios
- `given` MAY be an empty string (`""`)
- `when` MUST NOT be an empty string
- `then` MUST NOT be an empty string

Rationale:
- preserves the existing semantic model (spec → requirements → scenarios),
- makes CRUD possible without JSON array patching,
- keeps merge keys stable (`req_id`, `scenario.id`) for archive/sync.

Minimal example (main spec canonical blocks; these may be split across multiple fenced ` ```ison ` blocks in the Markdown file):

```ison
object.spec
version kind name purpose
1.0.0 llman.sdd.spec sample "Describe sample behavior."
```

```ison
table.requirements
req_id title statement
existing "Existing behavior" "System MUST preserve existing behavior."
```

```ison
table.scenarios
req_id id given when then
existing baseline "" "run sample" "behavior is preserved"
```

More examples (main specs):

1) Multiple requirements + multiple scenarios (with a non-empty `given` and multi-line `then`):

```ison
object.spec
version kind name purpose
1.0.0 llman.sdd.spec auth "Authentication behavior."
```

```ison
table.requirements
req_id title statement
login "User login" "System MUST allow valid users to login."
lockout "Account lockout" "System MUST lock out after repeated failures."
```

```ison
table.scenarios
req_id id given when then
login happy "" "user submits valid credentials" "session is created"
login bad_password "" "user submits invalid credentials" "login is rejected\nand no session is created"
lockout after_3 "user exists: alice" "user fails login 3 times" "account is locked"
```

2) Canonical blocks split under Markdown headings (headings/prose are outside; each canonical block name MUST appear exactly once):

````markdown
## Meta
```ison
object.spec
version kind name purpose
1.0.0 llman.sdd.spec sample "Describe sample behavior."
```

## Requirements
```ison
table.requirements
req_id title statement
existing "Existing behavior" "System MUST preserve existing behavior."
```

## Scenarios
```ison
table.scenarios
req_id id given when then
existing baseline "" "run sample" "behavior is preserved"
```
````

### 4) Canonical block schema for delta specs (ops + op_scenarios)
Inside the ` ```ison ` payload for `llmanspec/changes/<change>/specs/<capability>/spec.md`, define canonical blocks:

- `object.delta` (exactly 1 row; fields: `version kind`, kind = `llman.sdd.delta`)
- `table.ops` (fixed fields to keep dumps stable):
  - `op req_id title statement from to name`
  - unused fields MUST be `~` (null)
- `table.op_scenarios` (fields: `req_id id given when then`)

Block names MUST be strictly fixed to the identifiers above (no aliases).

Validation contract:
- `op` must be one of: `add_requirement`, `modify_requirement`, `remove_requirement`, `rename_requirement` (case-insensitive)
- add/modify MUST provide `req_id/title/statement` and may have scenarios via `table.op_scenarios`
- remove MUST provide `req_id` and may provide `name` (optional)
- rename MUST provide `req_id/from/to`
- strict mode:
  - add/modify MUST have ≥1 scenario row
  - remove/rename MUST NOT have scenario rows

Rationale:
- `ops` becomes a stable, row-based representation aligned with existing delta semantics,
- scenarios remain separately editable and can be appended without rewriting an op row.

Minimal example (delta spec canonical blocks):

```ison
object.delta
version kind
1.0.0 llman.sdd.delta

table.ops
op req_id title statement from to name
add_requirement added "Added behavior" "System MUST support the added behavior." ~ ~ ~

table.op_scenarios
req_id id given when then
added added_1 "" "a new action is taken" "the new behavior happens"
```

More examples (delta specs):

1) Full op matrix (add/modify/rename/remove) with scenarios only for add/modify:

```ison
object.delta
version kind
1.0.0 llman.sdd.delta

table.ops
op req_id title statement from to name
add_requirement added "Added behavior" "System MUST support the added behavior." ~ ~ ~
modify_requirement existing "Existing behavior" "System MUST preserve existing behavior (updated)." ~ ~ ~
rename_requirement login ~ ~ "login" "sign_in" ~
remove_requirement deprecated ~ ~ ~ ~ "Deprecated behavior"

table.op_scenarios
req_id id given when then
added happy "" "a new action is taken" "the new behavior happens"
existing baseline "" "run sample" "behavior is preserved\nand no errors are reported"
```

2) Ops and op_scenarios in separate fenced ` ```ison ` blocks (still canonical; block names are merged at runtime):

````markdown
## Ops
```ison
object.delta
version kind
1.0.0 llman.sdd.delta

table.ops
op req_id title statement from to name
add_requirement added "Added behavior" "System MUST support the added behavior." ~ ~ ~
```

## Scenarios
```ison
table.op_scenarios
req_id id given when then
added added_1 "" "a new action is taken" "the new behavior happens"
```
````

Invalid examples (must error):
- Duplicate canonical blocks: two `table.scenarios` blocks (or two `object.spec` blocks) in the same file.
- Unknown blocks: any non-canonical block name (for example `table.notes`) inside a fenced ` ```ison ` payload.
- Scenario schema violations: empty `when`/`then`, or `table.op_scenarios` rows keyed by a `req_id` that only has `rename/remove` ops.

### 5) Backward-compatible parsing path (transition)
Parser behavior (new command path: `llman sdd`):
- Extract all ` ```ison ` payloads as text (one file may contain multiple fenced blocks).
- Parse each payload using `ison-rs` (table/object ISON only).
- Merge blocks by block name into a single semantic document.
- Duplicate canonical block names are errors.
- Any block name outside the required canonical set MUST be treated as an error.

Legacy policy (`llman sdd-legacy`):
- `llman sdd-legacy` preserves the existing JSON-in-` ```ison ` parsing behavior (including any JSON repair fallback).

Validation policy:
- In `llman sdd`, JSON-in-` ```ison ` payloads MUST be treated as errors (not only in `--strict`) with an actionable hint to use `llman sdd-legacy` (and to rewrite the payload into canonical table/object ISON when ready).

Writer behavior:
- Any newly written/updated files in the new path (CRUD edits, archive merge writes) MUST emit table/object ISON (not JSON).

Rationale:
- keeps `llman sdd` free from legacy-format constraints,
- preserves a safe escape hatch (`llman sdd-legacy`) for existing repos,
- forces upgrades via explicit command selection rather than silent partial compatibility.

### 6) CLI authoring helpers (CRUD) are first-class commands
ISON authoring helpers MUST be integrated into the `llman sdd` workflow (not a separate “ison-only” command identity). Concretely, add explicit subcommands under `llman sdd` for:
- main spec skeleton generation (capability-level)
- delta spec skeleton generation (change + capability)
- delta CRUD operations (add/modify/remove/rename requirement ops)
- scenario append for add/modify ops
- lightweight metadata outputs for agents (so they can fetch a spec’s feature name/purpose without retrieving full requirement bodies)

Example shape (final naming to be implemented consistently across help text, templates, and docs):
- `llman sdd spec skeleton <capability>`
- `llman sdd delta skeleton <change-id> <capability>`
- `llman sdd delta add-op ...`
- `llman sdd delta add-scenario ...`
- `llman sdd show <capability> --type spec --json --meta-only` (and an opt-in compact JSON mode)

These commands operate by:
- parsing existing payload into a `Document`,
- applying deterministic edits,
- dumping back with stable ordering and optional pretty mode.

Rationale:
- reduces human error,
- makes agent instructions simpler (“use command X, then edit only the needed fields”),
- supports the “common operations” requirement without needing full editor integrations.

### 7) Default dump mode is token-friendly, with opt-in pretty alignment
- Default dumps MUST be token-friendly (no column alignment padding).
- Provide an opt-in flag (or mode) to pretty-align tables for review when desired.
- Dumps MUST remain deterministic (same input → same output) and preserve block ordering.

## Risks / Trade-offs

- [Risk] `ison-rs` parser behavior differs from the “prompt-optimizer” docs (e.g., no triple-quoted strings).
  - Mitigation: constrain the spec schema to values representable in `ison-rs` (scenario fields `given/when/then` use single quoted strings with `\n` escapes); document this in templates + validation hints.

- [Risk] Mixed-format repos (some JSON payloads, some table ISON) cause inconsistent diffs or confusing guidance.
  - Mitigation: explicit legacy command + template guidance that strongly prefers table ISON; `llman sdd` fails fast on JSON payloads with actionable next steps.

- [Risk] Dump formatting churn increases diffs.
  - Mitigation: stable block ordering; stable field ordering; default is token-friendly dumps; provide an opt-in pretty-alignment mode for review when needed.

- [Risk] Adding new CLI commands expands surface area and maintenance.
  - Mitigation: scope the first wave to skeleton + delta CRUD only; keep main spec CRUD as a follow-up if needed.

## Rollout Plan

1. Add `ison-rs` dependency and implement table/object ISON parsing + deterministic dumping.
2. Define and enforce schema validation for:
   - main spec: `object.spec` + `table.requirements` + `table.scenarios`
   - delta spec: `object.delta` + `table.ops` + `table.op_scenarios`
3. Implement dumping/writing of table ISON for any rewritten output path.
4. Add integrated authoring helpers under `llman sdd` (skeleton + delta CRUD).
6. Update `templates/sdd/**` and shared units (validation hints) to describe the new format and prefer CLI helpers.
7. Update integration tests to cover:
   - `llman sdd-legacy` continues to work on legacy JSON payloads,
   - `llman sdd` parses table/object ISON payloads and errors on legacy JSON payloads,
   - deterministic dumps (no churn) for new table/object ISON writes.
8. Rollout policy:
- `llman sdd` fails fast on legacy JSON payloads and points to `llman sdd-legacy` (and to canonical table/object ISON rewrite guidance).
- `llman sdd-legacy` remains available as the compatibility path during transition.

Rollback strategy:
- Users can temporarily use `llman sdd-legacy` if table ISON parsing has unexpected edge cases.
- Deterministic dumps ensure users do not experience churn when rewriting or editing table/object ISON payloads.

## Open Questions

1. Should delta specs continue to omit YAML frontmatter permanently, or should we optionally add it for symmetry?
2. Do we need main-spec CRUD commands (`spec add-requirement`, `spec add-scenario`) in the first iteration, or is delta-only CRUD sufficient for the initial rollout?

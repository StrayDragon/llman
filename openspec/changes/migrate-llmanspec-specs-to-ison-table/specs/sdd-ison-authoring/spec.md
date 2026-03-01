## ADDED Requirements

### Requirement: Canonical Table ISON Schema for Main Specs
`llmanspec/specs/<capability>/spec.md` MUST embed canonical semantic content using table/object ISON inside one or more fenced ` ```ison ` code blocks.

The ISON payload MUST provide these blocks with strictly fixed names and columns:
- `object.spec` (exactly 1 row): `version kind name purpose`
- `table.requirements`: `req_id title statement`
- `table.scenarios`: `req_id id given when then`

Validation MUST enforce:
- `object.spec.version` equals `"1.0.0"` (v1)
- `object.spec.kind` equals `llman.sdd.spec`
- in strict mode: `object.spec.name` equals `<capability>`
- every requirement has at least one scenario row
- `(req_id, id)` is unique across scenarios

`object.spec.name` is the stable feature-id for the spec. In strict mode, it MUST equal `<capability>`.

Scenario fields (`given`, `when`, `then`) MUST be ISON string values compatible with `ison-rs`. When quoting is required (spaces, punctuation, escapes), values MUST use **double quotes** (`"..."`). Newlines (when needed) MUST be represented using `\n` escapes (rather than multi-line string syntaxes).
  - `given` MAY be an empty string (`""`) when no precondition is needed.
  - `when` MUST NOT be an empty string.
  - `then` MUST NOT be an empty string.

Scenario semantics MUST be expressed via the structured columns (`given`/`when`/`then`), not by embedding legacy Markdown markers inside a single text blob.

Minimal authoring examples:

1) Main spec: YAML frontmatter (optional; shown as a snippet):

```yaml
---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - cargo test
---
```

2) Main spec: canonical blocks (may be placed in one or multiple fenced ` ```ison ` blocks, but each canonical block name MUST appear exactly once):

```ison
object.spec
version kind name purpose
"1.0.0" "llman.sdd.spec" sample "Describe sample behavior."
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

3) Delta spec: canonical blocks (unused fields are `~`; scenarios only for add/modify ops):

```ison
object.delta
version kind
"1.0.0" "llman.sdd.delta"

table.ops
op req_id title statement from to name
add_requirement added "Added behavior" "System MUST support the added behavior." ~ ~ ~
modify_requirement existing "Existing behavior" "System MUST preserve existing behavior (updated)." ~ ~ ~
rename_requirement renamed ~ ~ "old_id" "new_id" ~
remove_requirement removed ~ ~ ~ ~ "Removed behavior"

table.op_scenarios
req_id id given when then
added added_1 "" "a new action is taken" "the new behavior happens"
existing updated_1 "" "run sample" "behavior is preserved\nand no errors are reported"
```

#### Scenario: Canonical blocks can be parsed without depending on Markdown headings
- **WHEN** a main spec contains Markdown headings/prose plus the required ` ```ison ` blocks
- **THEN** the runtime extracts and parses the ISON blocks as the canonical semantic source
- **AND** command behavior does not depend on `##/###/####` heading conventions

### Requirement: Canonical Table ISON Schema for Delta Specs
`llmanspec/changes/<change-id>/specs/<capability>/spec.md` MUST embed delta semantics using table/object ISON inside one or more fenced ` ```ison ` code blocks.

The ISON payload MUST provide these blocks with strictly fixed names and columns:
- `object.delta` (exactly 1 row): `version kind` (kind = `llman.sdd.delta`)
- `table.ops`: `op req_id title statement from to name`
- `table.op_scenarios`: `req_id id given when then`

Validation MUST enforce:
- `object.delta.version` equals `"1.0.0"` (v1)
- `object.delta.kind` equals `llman.sdd.delta`

Unused fields in `table.ops` MUST be represented as `~` (null).

Scenario values in `table.op_scenarios` MUST follow the same encoding and style rules as main specs (ISON string; double quotes when quoting is required; newlines via `\n`; `given` MAY be empty; `when/then` MUST NOT be empty).

#### Scenario: Delta ops and scenarios are representable as deterministic tables
- **WHEN** a change contains an add/modify/remove/rename requirement delta spec
- **THEN** the delta operations are represented as rows in `table.ops`
- **AND** scenarios for add/modify are represented as rows in `table.op_scenarios` keyed by `req_id`

### Requirement: Multiple ISON Blocks Are Supported and Merged by Canonical Block Name
Spec and delta files MUST be allowed to split the required canonical blocks across multiple fenced ` ```ison ` code blocks (for example, one ISON block per Markdown section).

The runtime MUST:
- extract all ` ```ison ` code blocks from the file,
- parse each payload (table/object ISON),
- merge blocks by block name into a single semantic document,
- fail validation when a required canonical block is missing,
- fail validation when any canonical block name appears more than once,
- fail validation when any non-canonical block name is present.

Within a fenced ` ```ison ` block, the content MUST be valid ISON only. Markdown headings/prose MUST live outside the fenced block.

#### Scenario: Canonical blocks can be split across sections
- **WHEN** a spec file places `object.spec`, `table.requirements`, and `table.scenarios` in separate ` ```ison ` blocks under different Markdown headings
- **THEN** the runtime merges them and produces the same semantic result as a single combined ISON block

### Requirement: Token-Friendly Dumps Are the Default and Deterministic
All llman commands that write or rewrite ISON payloads (CRUD edits, archive merge outputs) MUST emit deterministic ISON dumps with stable block ordering.

Default dumps MUST be token-friendly (no column-alignment padding). The CLI MUST provide an opt-in mode/flag to pretty-align tables for review.

#### Scenario: Repeated writes do not churn formatting
- **WHEN** a maintainer runs the same write command twice without source changes
- **THEN** the emitted ` ```ison ` payload text is byte-identical across runs

### Requirement: CLI Provides Authoring Helpers for Skeletons and Delta CRUD
The SDD CLI MUST provide first-class authoring helpers to avoid manual editing of large ISON payloads:
- create/update a main spec skeleton for a capability
- create/update a delta spec skeleton for a change + capability
- add/modify/remove/rename a requirement op in a delta spec
- add a scenario to a delta op (keyed by `req_id` + `scenario.id`)

#### Scenario: Maintainer can author a delta spec without manual table editing
- **WHEN** a maintainer uses CLI authoring helpers to create a delta spec skeleton and add an `add_requirement` op plus a scenario
- **THEN** the resulting file validates in strict mode

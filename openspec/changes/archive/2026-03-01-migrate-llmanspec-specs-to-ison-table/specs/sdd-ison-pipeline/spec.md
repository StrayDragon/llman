## MODIFIED Requirements

### Requirement: Runtime Spec Parsing Uses ISON Container
The SDD runtime MUST parse `llmanspec` main specs from `spec.md` ISON container payloads rather than Markdown heading structure.

The runtime MUST treat table/object ISON as the canonical payload format, using strictly fixed canonical blocks:
- `object.spec`
- `table.requirements`
- `table.scenarios`

The runtime MUST interpret scenario semantics from structured scenario rows (for example: `req_id`, `id`, `given`, `when`, `then`) rather than from legacy Markdown markers embedded in free-form text.

Files MAY include Markdown headings/prose and MAY split canonical blocks across multiple fenced ` ```ison ` code blocks; the runtime MUST extract and merge all fenced ISON blocks by canonical block name.

#### Scenario: Show/list/validate parse main spec by ISON payload
- **WHEN** a user runs SDD commands that read `llmanspec/specs/<capability>/spec.md`
- **THEN** the parser extracts and parses the ` ```ison ` payload blocks as canonical semantic source
- **AND** command behavior does not depend on `##/###/####` heading conventions
- **AND** the parser supports multiple ` ```ison ` blocks and merges canonical blocks by block name

#### Scenario: Validation rejects legacy JSON payloads in new SDD command
- **WHEN** a user runs validation using `llman sdd` on a main spec whose ` ```ison ` payload is JSON
- **THEN** validation fails with non-zero exit
- **AND** the error message includes a concrete legacy-command hint (for example, `llman sdd-legacy validate ...`) and a hint to rewrite the payload into canonical table/object ISON

### Requirement: Runtime Delta Parsing Uses ISON Ops
The SDD runtime MUST parse change delta specs from table/object ISON ops blocks instead of Markdown section headers or JSON `ops[]`.

The runtime MUST read:
- delta ops from `table.ops`
- op scenarios from `table.op_scenarios`

and MUST key add/modify/remove/rename semantics by structured fields (including `req_id` and scenario `id`, plus scenario columns like `given/when/then`).

#### Scenario: Change validation parses ops table
- **WHEN** a user validates a change containing `llmanspec/changes/<change>/specs/<capability>/spec.md`
- **THEN** delta operations are read from `table.ops`
- **AND** scenarios for add/modify are read from `table.op_scenarios`
- **AND** add/modify/remove/rename semantics are keyed by structured fields (including `req_id`)

#### Scenario: Validation rejects legacy delta JSON payloads in new SDD command
- **WHEN** a user runs validation using `llman sdd` on a delta spec whose ` ```ison ` payload is JSON
- **THEN** validation fails with non-zero exit
- **AND** the error message includes a concrete legacy-command hint (for example, `llman sdd-legacy validate ...`) and a hint to rewrite the payload into canonical table/object ISON

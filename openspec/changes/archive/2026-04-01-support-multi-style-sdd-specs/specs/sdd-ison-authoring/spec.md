# sdd-ison-authoring Specification (Delta)

## ADDED Requirements

### Requirement: TOON 与 YAML 主 spec 使用统一对象语义
当 `llmanspec/config.yaml` 声明 `spec_style: toon` 或 `spec_style: yaml` 时，`llmanspec/specs/<capability>/spec.md` MUST 在与风格匹配的 fenced block 中承载一个 canonical 文档对象。

风格与 fence 的对应关系 MUST 为：

- `toon` → ` ```toon `
- `yaml` → ` ```yaml `

该 canonical 文档 MUST 表达以下主 spec 语义字段：

- `kind`
- `name`
- `purpose`
- `requirements`
- `scenarios`

其中：

- `kind` MUST 等于 `llman.sdd.spec`
- `name` 在 strict 模式下 MUST 等于 `<capability>`
- `requirements` 中的每一项 MUST 包含 `req_id`、`title`、`statement`
- `scenarios` 中的每一项 MUST 包含 `req_id`、`id`、`given`、`when`、`then`
- 每个 requirement MUST 至少有一个 scenario
- `(req_id, id)` MUST 在 scenarios 中唯一
- `given` MAY 为空字符串
- `when` 与 `then` MUST NOT 为空字符串

#### Scenario: YAML 主 spec 满足统一对象语义
- **WHEN** 项目声明 `spec_style: yaml`
- **AND** `llmanspec/specs/sample/spec.md` 包含 ` ```yaml ` canonical payload
- **THEN** 运行时可将其解析为主 spec 语义对象

#### Scenario: TOON 主 spec 缺少必需字段会失败
- **WHEN** 项目声明 `spec_style: toon`
- **AND** `llmanspec/specs/sample/spec.md` 的 TOON payload 缺少 `requirements`
- **THEN** `llman sdd validate sample --type spec --strict` 返回非零

### Requirement: TOON 与 YAML delta spec 使用统一 ops 语义
当 `llmanspec/config.yaml` 声明 `spec_style: toon` 或 `spec_style: yaml` 时，`llmanspec/changes/<change-id>/specs/<capability>/spec.md` MUST 在与风格匹配的 fenced block 中承载一个 canonical delta 文档对象。

该对象 MUST 表达以下字段：

- `kind`
- `ops`
- `op_scenarios`

其中：

- `kind` MUST 等于 `llman.sdd.delta`
- `ops` 中的每一项 MUST 使用字段 `op`、`req_id`、`title`、`statement`、`from`、`to`、`name`
- `op_scenarios` 中的每一项 MUST 使用字段 `req_id`、`id`、`given`、`when`、`then`
- `op` MUST 是 `add_requirement`、`modify_requirement`、`remove_requirement`、`rename_requirement` 之一
- strict 模式下，`add_requirement` 与 `modify_requirement` MUST 至少有一个 scenario
- strict 模式下，`remove_requirement` 与 `rename_requirement` MUST NOT 带 scenario

#### Scenario: YAML delta spec 可表达 add/modify/remove/rename
- **WHEN** 项目声明 `spec_style: yaml`
- **THEN** 运行时可以从 `ops` 与 `op_scenarios` 中读取 add/modify/remove/rename 语义

#### Scenario: TOON delta spec 为 rename op 携带 scenario 会失败
- **WHEN** 项目声明 `spec_style: toon`
- **AND** 某个 `rename_requirement` op 对应了 `op_scenarios`
- **THEN** `llman sdd validate <change-id> --type change --strict` 返回非零

## MODIFIED Requirements

### Requirement: Canonical Table ISON Schema for Main Specs
当 `llmanspec/config.yaml` 声明 `spec_style: ison` 时，`llmanspec/specs/<capability>/spec.md` MUST embed canonical semantic content using table/object ISON inside one or more fenced ` ```ison ` code blocks.

The ISON payload MUST provide these blocks with strictly fixed names and columns:
- `object.spec` (exactly 1 row): `kind name purpose`
- `table.requirements`: `req_id title statement`
- `table.scenarios`: `req_id id given when then`

Validation MUST enforce:
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

#### Scenario: Canonical blocks can be parsed without depending on Markdown headings
- **WHEN** a main spec in an `ison` project contains Markdown headings/prose plus the required ` ```ison ` blocks
- **THEN** the runtime extracts and parses the ISON blocks as the canonical semantic source
- **AND** command behavior does not depend on `##/###/####` heading conventions

### Requirement: Canonical Table ISON Schema for Delta Specs
当 `llmanspec/config.yaml` 声明 `spec_style: ison` 时，`llmanspec/changes/<change-id>/specs/<capability>/spec.md` MUST embed delta semantics using table/object ISON inside one or more fenced ` ```ison ` code blocks.

The ISON payload MUST provide these blocks with strictly fixed names and columns:
- `object.delta` (exactly 1 row): `kind` (kind = `llman.sdd.delta`)
- `table.ops`: `op req_id title statement from to name`
- `table.op_scenarios`: `req_id id given when then`

Validation MUST enforce:
- `object.delta.kind` equals `llman.sdd.delta`

Unused fields in `table.ops` MUST be represented as `~` (null).

Scenario values in `table.op_scenarios` MUST follow the same encoding and style rules as main specs (ISON string; double quotes when quoting is required; newlines via `\n`; `given` MAY be empty; `when/then` MUST NOT be empty).

#### Scenario: Delta ops and scenarios are representable as deterministic tables
- **WHEN** an `ison` project contains an add/modify/remove/rename requirement delta spec
- **THEN** the delta operations are represented as rows in `table.ops`
- **AND** scenarios for add/modify are represented as rows in `table.op_scenarios` keyed by `req_id`

### Requirement: Multiple ISON Blocks Are Supported and Merged by Canonical Block Name
当 `llmanspec/config.yaml` 声明 `spec_style: ison` 时，spec and delta files MUST be allowed to split the required canonical blocks across multiple fenced ` ```ison ` code blocks (for example, one ISON block per Markdown section).

The runtime MUST:
- extract all ` ```ison ` code blocks from the file,
- parse each payload (table/object ISON),
- merge blocks by block name into a single semantic document,
- fail validation when a required canonical block is missing,
- fail validation when any canonical block name appears more than once,
- fail validation when any non-canonical block name is present.

Within a fenced ` ```ison ` block, the content MUST be valid ISON only. Markdown headings/prose MUST live outside the fenced block.

#### Scenario: Canonical blocks can be split across sections
- **WHEN** an `ison` spec file places `object.spec`, `table.requirements`, and `table.scenarios` in separate ` ```ison ` blocks under different Markdown headings
- **THEN** the runtime merges them and produces the same semantic result as a single combined ISON block

### Requirement: Token-Friendly Dumps Are the Default and Deterministic
All llman commands that write or rewrite spec payloads (CRUD edits, archive merge outputs, conversions) MUST emit deterministic dumps for the configured style.

- In `ison` projects, the default dump MUST remain token-friendly (no column-alignment padding).
- In `ison` projects, the CLI MAY provide opt-in pretty alignment for review.
- In `toon` and `yaml` projects, the serializer MUST use stable field ordering, stable list ordering, and consistent indentation.

Repeated writes without semantic changes MUST be byte-identical within the same configured style.

#### Scenario: Repeated YAML writes do not churn formatting
- **WHEN** a maintainer runs the same YAML-targeted write command twice without source changes
- **THEN** the emitted ` ```yaml ` payload text is byte-identical across runs

### Requirement: CLI Provides Authoring Helpers for Skeletons and Delta CRUD
The SDD CLI MUST provide first-class authoring helpers that emit the project’s configured style for:

- create/update a main spec skeleton for a capability
- create/update a delta spec skeleton for a change + capability
- add/modify/remove/rename a requirement op in a delta spec
- add a scenario to a spec or delta op

If the project has no declared `spec_style`, these authoring helpers MUST fail loudly instead of silently choosing a format. Style-specific formatting flags MUST also be enforced strictly: `--pretty-ison` MUST only be accepted for `ison` projects.

#### Scenario: Maintainer can author a YAML delta spec without manual format switching
- **WHEN** a maintainer in a `yaml` project uses CLI authoring helpers to create a delta spec skeleton and add an `add_requirement` op plus a scenario
- **THEN** the resulting file is written in ` ```yaml ` format
- **AND** the file validates in strict mode

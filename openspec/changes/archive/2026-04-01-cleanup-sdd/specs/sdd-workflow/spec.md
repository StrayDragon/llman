# sdd-workflow Specification (Delta)

## ADDED Requirements

### Requirement: SDD Propose Skill
`llman-sdd-propose` skill MUST guide an AI assistant to create a new change and generate all planning artifacts in one pass (proposal + delta specs + tasks; design optional), then run validation and suggest next actions.

#### Scenario: Propose creates a change and artifacts
- **WHEN** a user invokes `llman-sdd-propose` with a change description (and/or a change id)
- **THEN** the assistant creates `llmanspec/changes/<change-id>/` with `proposal.md`, `specs/**`, and `tasks.md` (and `design.md` when needed)
- **AND** the assistant runs `llman sdd validate <change-id> --strict --no-interactive`
- **AND** the assistant suggests `llman-sdd-apply` for implementation

## MODIFIED Requirements

### Requirement: CLI Provides Lightweight Spec Metadata for Agents
The CLI MUST provide an agent-friendly way to fetch a spec’s feature name/purpose without retrieving full requirement bodies.

At minimum, `llman sdd show` MUST support a JSON metadata-only mode for specs:
- command shape: `llman sdd show <spec-id> --type spec --json --meta-only`
- output MUST include:
  - `id` (spec id / directory name)
  - `featureId` (from `object.spec.name`)
  - `title` (human-facing name; defaults to `featureId`)
  - `overview` (from `object.spec.purpose`)
  - `requirementCount`
  - `metadata`
- output MUST NOT include `requirements` when `--meta-only` is set.

If the CLI provides a `--compact-json` mode, it MUST emit JSON without pretty formatting whitespace (token-friendly) while keeping field order stable.
This MUST apply to `llman sdd list/show/validate` whenever `--json` is used.

#### Scenario: Agent fetches spec feature name cheaply
- **WHEN** an agent needs the spec feature name/purpose for prompt assembly
- **THEN** the agent can call `llman sdd show <spec-id> --type spec --json --meta-only`
- **AND** the output includes only spec metadata (no `requirements` array)

### Requirement: Validation Fails on JSON-in-ISON Payloads
`llman sdd validate` MUST fail when any `llmanspec` main spec or delta spec contains a fenced ` ```ison ` payload that is JSON.

The error MUST be actionable and MUST include:
- a concrete hint to rewrite the payload into canonical table/object ISON (for example: main spec uses `object.spec` + `table.requirements` + `table.scenarios`; delta spec uses `object.delta` + `table.ops` + `table.op_scenarios`).

#### Scenario: Validate blocks legacy payloads
- **WHEN** a user runs `llman sdd validate` (with or without `--strict`) on a project containing JSON-in-` ```ison ` payloads
- **THEN** validation fails with non-zero exit
- **AND** output includes explicit canonical rewrite guidance (no legacy-command hint)

### Requirement: No Automatic Migration Is Required for Legacy Repos
The system MUST enforce canonical table/object ISON for `llmanspec` specs and deltas.

The system MUST NOT require (or depend on) an automatic migration command as part of the workflow. Users MUST manually rewrite legacy JSON-in-` ```ison ` payloads into the canonical table/object ISON schema before using `llman sdd` successfully.

#### Scenario: User chooses when to switch formats
- **WHEN** a repository remains on legacy JSON-in-` ```ison ` payloads
- **THEN** `llman sdd validate` fails with a clear canonical rewrite hint
- **AND** no automatic migration command is required or implied

## REMOVED Requirements

### Requirement: Style Routing for SDD Commands
`llman sdd` command flows MUST support explicit style selection for new vs legacy tracks.

**Reason**: legacy track 被移除后，不再存在 new vs legacy 的 style 分流需求。

**Migration**: `llman sdd` 始终使用 new-style 语义与 `templates/sdd/**` 模板；移除所有 legacy style selector 与分支代码。

### Requirement: Default Style Is New
The default SDD style MUST be new when style selector is omitted.

**Reason**: style selector 被移除后，该要求不再有意义（系统始终是 new-style）。

**Migration**: `llman sdd` 永远以 new-style 运作，不提供 legacy override。

### Requirement: Legacy Workflow Command Preserves JSON Parsing
The CLI MUST provide a legacy workflow command to preserve the previous JSON-in-` ```ison ` parsing behavior for `llmanspec` specs and delta specs.

**Reason**: legacy JSON-in-` ```ison ` 语义与 `llman sdd-legacy` 命令组被彻底移除，减少复杂度与维护成本。

**Migration**: 任何仍包含 legacy JSON payload 的仓库必须手工重写为 canonical table/object ISON 才能继续使用 `llman sdd`。

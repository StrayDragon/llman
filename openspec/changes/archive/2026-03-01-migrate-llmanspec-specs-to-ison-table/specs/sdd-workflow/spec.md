## ADDED Requirements

### Requirement: Legacy Workflow Command Preserves JSON Parsing
The CLI MUST provide a legacy workflow command to preserve the previous JSON-in-` ```ison ` parsing behavior for `llmanspec` specs and delta specs.

This legacy command MUST:
- be exposed as `llman sdd-legacy ...`
- preserve the previous parsing/validation semantics for legacy repositories
- continue to support existing repositories whose ` ```ison ` payload is JSON (including any JSON repair behavior already present)
- include the workflow subcommands referenced by `templates/sdd-legacy/**` (at minimum: `init`, `update`, `update-skills`, `list`, `show`, `validate`, and `archive` including archive subcommands like freeze/thaw as applicable)

`llman sdd` (the new default) MUST be allowed to evolve without needing to accept legacy JSON payloads.

#### Scenario: Legacy repo remains usable via sdd-legacy
- **WHEN** a repository contains JSON-in-` ```ison ` payloads under `llmanspec/specs/**` or `llmanspec/changes/**/specs/**`
- **THEN** `llman sdd-legacy show/validate/archive` continues to operate successfully
- **AND** `llman sdd show/validate/archive` fails fast with a legacy-command hint and canonical rewrite guidance

### Requirement: SDD Provides ISON Authoring Commands
`llman sdd` MUST provide an explicit command group for ISON spec authoring/editing to reduce manual edits of `llmanspec/**/spec.md`.

At minimum, the CLI MUST support:
- generating a main spec skeleton for a capability
- adding a requirement to a main spec
- adding a scenario to a main spec (keyed by `req_id` + `scenario.id`)
- generating a delta spec skeleton for a change + capability
- adding/removing/updating delta ops (add/modify/remove/rename requirement)
- adding scenarios for add/modify ops (keyed by `req_id` + `scenario.id`)

#### Scenario: Generate delta skeleton and add an op
- **WHEN** a maintainer creates a new change directory and needs to add a delta requirement
- **THEN** the maintainer can use the CLI to generate a delta spec skeleton and add an op without manual table editing

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
This MUST apply to `llman sdd list/show/validate` (and `llman sdd-legacy list/show/validate`) whenever `--json` is used.

#### Scenario: Agent fetches spec feature name cheaply
- **WHEN** an agent needs the spec feature name/purpose for prompt assembly
- **THEN** the agent can call `llman sdd show <spec-id> --type spec --json --meta-only`
- **AND** the output includes only spec metadata (no `requirements` array)

### Requirement: Validation Fails on JSON-in-ISON Payloads
`llman sdd validate` MUST fail when any `llmanspec` main spec or delta spec contains a fenced ` ```ison ` payload that is JSON.

The error MUST be actionable and MUST include:
- a concrete hint to temporarily use the legacy command (`llman sdd-legacy ...`).

#### Scenario: Validate blocks legacy payloads
- **WHEN** a user runs `llman sdd validate` (with or without `--strict`) on a project containing JSON-in-` ```ison ` payloads
- **THEN** validation fails with non-zero exit
- **AND** output includes an explicit legacy-command hint

### Requirement: No Automatic Migration Is Required for Legacy Repos
The system MUST support a “two command” posture:
- `llman sdd` enforces canonical table/object ISON.
- `llman sdd-legacy` preserves the legacy JSON parsing behavior.

The system MUST NOT require an automatic migration command as part of the new-style workflow. Users MAY manually rewrite legacy payloads into the canonical table/object ISON schema when they choose to switch from `llman sdd-legacy` to `llman sdd`.

#### Scenario: User chooses when to switch formats
- **WHEN** a repository remains on legacy JSON payloads
- **THEN** users can continue using `llman sdd-legacy`
- **AND** switching to `llman sdd` requires rewriting payloads into canonical table/object ISON, but no automatic migration command is required

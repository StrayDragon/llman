## ADDED Requirements

### Requirement: Style Routing for SDD Commands
`llman sdd` command flows MUST support explicit style selection for new vs legacy tracks.

#### Scenario: Update commands accept style selection
- **WHEN** a user runs `llman sdd update` or `llman sdd update-skills` with style selector
- **THEN** the command routes template loading and output generation through the selected style track

### Requirement: Default Style Is New
The default SDD style MUST be new when style selector is omitted.

#### Scenario: Show and validate default to new style
- **WHEN** a user runs `llman sdd show` or `llman sdd validate` without style selector
- **THEN** the command evaluates and displays new style outputs by default

### Requirement: Archive Merge Emits ISON Spec Payload
`llman sdd archive` MUST merge delta changes into main specs using structured ISON semantics and emit ISON payload output.

#### Scenario: Archive applies ops and writes merged ISON
- **WHEN** a change contains delta spec ops and a user runs `llman sdd archive <change>`
- **THEN** archive applies add/modify/remove/rename operations over requirement ids
- **AND** the resulting `llmanspec/specs/<capability>/spec.md` contains merged ISON payload as canonical spec content

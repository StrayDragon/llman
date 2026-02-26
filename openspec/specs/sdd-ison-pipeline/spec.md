# sdd-ison-pipeline Specification

## Purpose
TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive.
## Requirements
### Requirement: ISON-First SDD Template Sources
The SDD template system MUST support ISON source templates as the primary authoring format for the new style track.

#### Scenario: New style template generation reads ISON source
- **WHEN** a maintainer runs SDD template refresh for new style
- **THEN** the system reads ISON source templates
- **AND** renders Markdown outputs used by generated instructions and skills

### Requirement: ISON Validation Before Render
The system MUST validate ISON source templates before rendering outputs.

#### Scenario: Invalid ISON source blocks rendering
- **WHEN** a new style ISON template has structural or type errors
- **THEN** SDD template generation fails with non-zero exit
- **AND** no partial rendered output is written for that failed template

### Requirement: Runtime Spec Parsing Uses ISON Container
The SDD runtime MUST parse `llmanspec` main specs from `spec.md` ISON container payloads rather than Markdown heading structure.

#### Scenario: Show/list/validate parse main spec by ISON payload
- **WHEN** a user runs SDD commands that read `llmanspec/specs/<capability>/spec.md`
- **THEN** the parser extracts the ` ```ison ` payload as canonical semantic source
- **AND** command behavior does not depend on `##/###/####` heading conventions

### Requirement: Runtime Delta Parsing Uses ISON Ops
The SDD runtime MUST parse change delta specs from ISON `ops[]` instead of Markdown section headers.

#### Scenario: Change validation parses ops array
- **WHEN** a user validates a change containing `llmanspec/changes/<change>/specs/<capability>/spec.md`
- **THEN** delta operations are read from `ops[]`
- **AND** add/modify/remove/rename semantics are keyed by structured fields (including `req_id`)

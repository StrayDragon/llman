# sdd-legacy-compat Specification

## Purpose
TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive.
## Requirements
### Requirement: Legacy Track Must Remain Available
The SDD workflow MUST keep a legacy style track for templates/skills/prompts that users can explicitly select.

#### Scenario: User selects legacy style explicitly
- **WHEN** a user runs SDD generation with legacy style option
- **THEN** output is generated from legacy templates
- **AND** no new-style-only validation constraints are enforced on that legacy output path

### Requirement: New Style Default with Explicit Legacy Override
The SDD workflow MUST default to new style generation and behavior unless legacy is explicitly requested.

#### Scenario: No style flag uses new track
- **WHEN** a user runs SDD generation commands without a style selector
- **THEN** the system uses the new style templates by default
- **AND** generated outputs include the new structured governance behavior

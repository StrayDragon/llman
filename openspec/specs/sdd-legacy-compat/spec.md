# sdd-legacy-compat Specification

## Purpose
TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive.
## Requirements
### Requirement: Legacy Track Is Retired
The SDD workflow MUST NOT provide a legacy track (`sdd-legacy`) for templates/skills/prompts.

#### Scenario: User tries to use legacy track
- **WHEN** a user looks for or attempts to use legacy-style SDD commands or templates
- **THEN** the system fails loudly
- **AND** the error explains that only the canonical new-style workflow is supported

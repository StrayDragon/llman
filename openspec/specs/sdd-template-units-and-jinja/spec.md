# sdd-template-units-and-jinja Specification

## Purpose
TBD - created by archiving change refactor-sdd-template-units-minijinja. Update Purpose after archive.
## Requirements
### Requirement: SDD Template Units Must Be Independent and Discoverable
SDD prompt composition MUST split reusable prompt fragments into independent template-unit files, with explicit unit identifiers and deterministic lookup rules by locale.

#### Scenario: Unit file can be edited independently
- **WHEN** a maintainer updates one prompt unit used by multiple SDD templates
- **THEN** the change is made in a single unit file without editing unrelated templates

#### Scenario: Locale-scoped unit resolution is deterministic
- **WHEN** the renderer resolves a unit for `zh-Hans` with fallback to `en`
- **THEN** it follows a documented deterministic fallback chain and returns exactly one resolved unit source

### Requirement: SDD Rendering Must Use MiniJinja Injection Contracts
SDD template rendering MUST use MiniJinja-based injection for unit composition, and rendering MUST fail fast on missing unit references or missing required variables.

#### Scenario: Missing unit reference fails rendering
- **WHEN** a template references a unit identifier that does not exist
- **THEN** render operation exits non-zero with a clear missing-unit error

#### Scenario: Missing required render variable fails rendering
- **WHEN** a template requires a render variable that is not provided
- **THEN** render operation exits non-zero and identifies the missing variable

### Requirement: Rendered Outputs Must Stay Stable and Self-Contained
Rendered SDD templates and skills MUST remain self-contained text artifacts and MUST preserve stable output ordering to reduce maintenance diff noise.

#### Scenario: Generated SKILL output remains self-contained
- **WHEN** user runs `llman sdd update-skills --no-interactive --tool codex`
- **THEN** generated `SKILL.md` files include fully rendered content without unresolved injection markers

#### Scenario: Stable generation order
- **WHEN** user runs the same generation command twice without source changes
- **THEN** generated file content order is identical across runs

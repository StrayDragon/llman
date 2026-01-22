# cursor-export Specification

## Purpose
TBD - created by archiving change update-cli-quality-specs. Update Purpose after archive.
## Requirements
### Requirement: Non-interactive database selection order
Non-interactive cursor export MUST honor `db_path` first, then `workspace_dir` resolution, then auto-discovery when neither is provided.

#### Scenario: db_path provided
- **WHEN** `db_path` is set in non-interactive mode
- **THEN** the export uses that database path directly

#### Scenario: workspace_dir provided
- **WHEN** `workspace_dir` is set and `db_path` is not provided
- **THEN** the export resolves the workspace to its database path and uses it

#### Scenario: no overrides
- **WHEN** neither `db_path` nor `workspace_dir` is provided
- **THEN** the export selects a database path via auto-discovery

### Requirement: Output mode validation
Unsupported `output_mode` values MUST return an error instead of printing and continuing.

#### Scenario: Unsupported output mode
- **WHEN** `output_mode` is not one of the supported values
- **THEN** the command returns an error that is surfaced by the CLI entrypoint

### Requirement: File output location rules
File-based output modes MUST follow deterministic output locations.

#### Scenario: output_mode=single-file
- **WHEN** `output_mode` is `single-file`
- **THEN** the export writes to `output_file` if provided, otherwise defaults to `cursor_export.md`

#### Scenario: output_mode=file
- **WHEN** `output_mode` is `file`
- **THEN** the export writes into the `output_file` directory if provided, otherwise defaults to `./cursor_exports`


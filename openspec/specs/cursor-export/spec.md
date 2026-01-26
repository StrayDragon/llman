# cursor-export Specification

## Purpose
Describe Cursor export selection and output behavior (interactive and non-interactive).
## Requirements
### Requirement: Non-interactive database selection order
Non-interactive cursor export MUST honor `db_path` first, then `workspace_dir` resolution, then auto-discovery when neither is provided. Auto-discovery selects the latest workspace database, preferring a database that contains chat/composer data.

#### Scenario: db_path provided
- **WHEN** `db_path` is set in non-interactive mode
- **THEN** the export uses that database path directly

#### Scenario: workspace_dir provided
- **WHEN** `workspace_dir` is set and `db_path` is not provided
- **THEN** the export resolves the workspace to its database path and uses it
- **AND** an error is returned if the directory does not exist or is not found among known workspaces

#### Scenario: no overrides
- **WHEN** neither `db_path` nor `workspace_dir` is provided
- **THEN** the export selects a database path via auto-discovery

### Requirement: Non-interactive selection defaults
When `composer_id` is provided, the export MUST target only that composer conversation; otherwise it MUST export all conversations from the resolved database.

#### Scenario: composer_id provided
- **WHEN** `composer_id` is set in non-interactive mode
- **THEN** the export writes only the matching composer conversation or returns an error if not found

#### Scenario: no composer_id
- **WHEN** `composer_id` is not provided in non-interactive mode
- **THEN** the export writes all available conversations

### Requirement: Output mode validation
`output_mode` MUST be one of `console`, `file`, or `single-file` (default `console`). Unsupported values MUST return an error instead of printing and continuing.

#### Scenario: Unsupported output mode
- **WHEN** `output_mode` is not one of the supported values
- **THEN** the command returns an error that is surfaced by the CLI entrypoint

### Requirement: File output location rules
File-based output modes MUST follow deterministic output locations and naming.

#### Scenario: output_mode=single-file
- **WHEN** `output_mode` is `single-file`
- **THEN** the export writes to `output_file` if provided, otherwise defaults to `cursor_conversations.md`

#### Scenario: output_mode=file
- **WHEN** `output_mode` is `file`
- **THEN** the export writes into the `output_file` directory if provided, otherwise defaults to `./cursor_exports`
- **AND** filenames are numbered and sanitized (`NN_<title>.md` with non-alphanumeric characters removed and spaces converted to underscores)

# errors-exit Specification

## Purpose
TBD - created by archiving change update-cli-quality-specs. Update Purpose after archive.
## Requirements
### Requirement: Centralized error output and exit codes
The CLI entrypoint MUST render a single user-facing error message to stderr and exit with code 1 when any command returns an error.

#### Scenario: Command failure
- **WHEN** a subcommand returns an error
- **THEN** the CLI prints one localized error line to stderr and exits with code 1

### Requirement: Error propagation from command handlers
Command handlers MUST return `Err` on failure instead of printing errors and returning success.

#### Scenario: Subcommand failure path
- **WHEN** a command encounters an operational error
- **THEN** it propagates the error to `main()` without printing success output


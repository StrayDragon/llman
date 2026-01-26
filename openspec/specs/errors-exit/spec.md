# errors-exit Specification

## Purpose
Define llman CLI error rendering and exit behavior.
## Requirements
### Requirement: Entrypoint error rendering
The CLI entrypoint MUST render a single user-facing error message to stderr and exit with code 1 when a command returns an error. `LlmanError` values MUST be localized before being wrapped by `messages.error`; other errors use `to_string()`.

#### Scenario: Command failure
- **WHEN** a subcommand returns an error
- **THEN** the CLI prints one localized error line to stderr and exits with code 1

### Requirement: Subcommand error handling
Command handlers MUST return `Err` on fatal failures. Interactive flows MAY print their own error messages and exit directly, and recoverable issues MAY be logged to stderr without failing the command.

#### Scenario: Non-interactive show without item
- **WHEN** `llman sdd show` runs without an item in a non-interactive terminal
- **THEN** it prints the non-interactive hint to stderr and exits with code 1

#### Scenario: Recoverable warning
- **WHEN** a configured skills source path does not exist
- **THEN** a warning is printed to stderr and the sync continues

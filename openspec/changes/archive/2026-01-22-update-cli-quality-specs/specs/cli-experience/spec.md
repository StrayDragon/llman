# Capability: cli-experience

## ADDED Requirements
### Requirement: Centralized user-facing messages
User-facing strings MUST be sourced from `locales/app.yml` via localization helpers, and message keys MUST remain English-only placeholders unless explicitly requested.

#### Scenario: Standard message lookup
- **WHEN** a command emits a user-facing message
- **THEN** the message is retrieved from `locales/app.yml` using a localization key

### Requirement: Consistent stdout/stderr usage
Normal command output MUST go to stdout, and errors or warnings MUST go to stderr.

#### Scenario: Operational failure
- **WHEN** a command fails during execution
- **THEN** the user-facing error is written to stderr only

### Requirement: Avoid mixed formatting within a single line
Single-line messages MUST avoid mixing unrelated formatting styles or ad-hoc prefixes.

#### Scenario: User-facing label
- **WHEN** a single-line status or label is printed
- **THEN** the line uses consistent formatting and does not mix unrelated prefixes

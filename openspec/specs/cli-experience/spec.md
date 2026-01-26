# cli-experience Specification

## Purpose
Describe llman CLI messaging, localization coverage, and stdout/stderr conventions.
## Requirements
### Requirement: Localized runtime messaging with documented exceptions
Runtime prompts, status lines, and errors MUST use `t!` keys from `locales/app.yml` when a suitable key exists. When no localization key exists, commands MAY emit inline strings (for example: task count labels, on/off markers, separators, or generated export content).

#### Scenario: Localized prompt with inline formatting
- **WHEN** a command prompts the user or prints a status header
- **THEN** the primary message text is resolved from `locales/app.yml` and any inline markers (emoji, bullets, separators) are embedded as literals

#### Scenario: Inline-only content
- **WHEN** a command outputs generated content (such as exported markdown or file name labels)
- **THEN** the generated content may include hard-coded text that is not localized

### Requirement: Locale is fixed to English
The CLI MUST set the locale to English at startup and use the `locales/app.yml` bundle.

#### Scenario: CLI startup
- **WHEN** the CLI launches
- **THEN** the runtime locale is set to `en` and localization keys resolve against `locales/app.yml`

### Requirement: Consistent stdout/stderr usage
Normal command output and interactive prompts MUST go to stdout. Errors SHOULD be written to stderr; non-fatal notices MAY still use stdout.

#### Scenario: Operational failure
- **WHEN** a command fails during execution
- **THEN** the user-facing error is written to stderr

#### Scenario: Progress output
- **WHEN** a command reports progress or results
- **THEN** the messages are written to stdout

### Requirement: Consistent single-line formatting
Single-line messages MUST use a single consistent prefix or label; inline emoji or separators MAY be included but MUST avoid mixing unrelated prefixes.

#### Scenario: User-facing label
- **WHEN** a single-line status or label is printed
- **THEN** the line uses consistent formatting and does not mix unrelated prefixes

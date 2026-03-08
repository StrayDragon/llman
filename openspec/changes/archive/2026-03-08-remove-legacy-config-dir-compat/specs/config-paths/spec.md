# config-paths Specification (Delta)

## MODIFIED Requirements

### Requirement: Config directory resolution precedence
The CLI MUST resolve the configuration directory with the following precedence: CLI `--config-dir` override, `LLMAN_CONFIG_DIR` environment variable, then the default path `~/.config/llman`. The resolved value MUST be assigned to `LLMAN_CONFIG_DIR` for all subcommands, and resolution MUST NOT create directories.

#### Scenario: CLI override provided
- **WHEN** the user runs a command with `--config-dir`
- **THEN** the resolved config directory is the CLI value and `LLMAN_CONFIG_DIR` is set for subcommands

#### Scenario: Env override provided
- **WHEN** `LLMAN_CONFIG_DIR` is set and no CLI override is provided
- **THEN** the resolved config directory is the env value

#### Scenario: Default fallback
- **WHEN** no CLI or env override is provided
- **THEN** the resolved config directory is `<home>/.config/llman` and resolution does not create directories

#### Scenario: Legacy macOS directories are ignored by default resolution
- **WHEN** the user runs a command without CLI/env overrides on macOS, and `<home>/Library/Application Support/llman` or `<home>/Library/Application Support/com.StrayDragon.llman` contains a recognizable config root
- **THEN** the resolved config directory is `<home>/.config/llman`
- **AND** the CLI does not print a legacy migration warning
- **AND** `LLMAN_CONFIG_DIR` is set to `<home>/.config/llman`

#### Scenario: Env propagation
- **WHEN** the CLI resolves a config directory
- **THEN** `LLMAN_CONFIG_DIR` is set to the resolved value for the process

## REMOVED Requirements

### Requirement: macOS legacy config compatibility warning
On macOS, when neither CLI `--config-dir` nor `LLMAN_CONFIG_DIR` override is provided, the CLI MUST support compatibility with legacy config directories and MUST warn when it resolves to a legacy directory.

**Reason**: The migration window has passed, and continuing to auto-detect legacy macOS config directories makes default resolution depend on leftover filesystem state, which adds maintenance cost and user confusion.

**Migration**: Users who still keep config under `<home>/Library/Application Support/llman` or `<home>/Library/Application Support/com.StrayDragon.llman` MUST either move that config into `<home>/.config/llman` or explicitly set `LLMAN_CONFIG_DIR` / `--config-dir` to the legacy path.

# config-paths Specification (Delta)

## ADDED Requirements

### Requirement: macOS legacy config compatibility warning
On macOS, when neither CLI `--config-dir` nor `LLMAN_CONFIG_DIR` override is provided, the CLI MUST support compatibility with legacy config directories and MUST warn when it resolves to a legacy directory.

Legacy directories (candidates) are:
- `<home>/Library/Application Support/llman`
- `<home>/Library/Application Support/com.StrayDragon.llman`

The resolver MUST NOT create directories as part of resolution, even when legacy compatibility logic is applied.

#### Scenario: Legacy config directory is selected
- **WHEN** the user runs a command without CLI/env overrides on macOS, and `~/.config/llman` does not contain a recognizable config root, and `<home>/Library/Application Support/llman` contains a recognizable config root
- **THEN** the resolved config directory is `<home>/Library/Application Support/llman`, and the CLI prints a migration warning to stderr recommending `~/.config/llman`, and `LLMAN_CONFIG_DIR` is set to the legacy path

#### Scenario: Legacy bundle-id config directory is selected
- **WHEN** the user runs a command without CLI/env overrides on macOS, and `~/.config/llman` does not contain a recognizable config root, and `<home>/Library/Application Support/llman` does not contain a recognizable config root, and `<home>/Library/Application Support/com.StrayDragon.llman` contains a recognizable config root
- **THEN** the resolved config directory is `<home>/Library/Application Support/com.StrayDragon.llman`, and the CLI prints a migration warning to stderr recommending `~/.config/llman`, and `LLMAN_CONFIG_DIR` is set to the legacy path

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
- **WHEN** no CLI or env override is provided, and the macOS legacy compatibility rule does not select a legacy directory
- **THEN** the resolved config directory is `<home>/.config/llman` and resolution does not create directories

#### Scenario: Env propagation
- **WHEN** the CLI resolves a config directory
- **THEN** `LLMAN_CONFIG_DIR` is set to the resolved value for the process

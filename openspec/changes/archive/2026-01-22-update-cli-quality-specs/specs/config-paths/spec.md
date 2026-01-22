# Capability: config-paths

## ADDED Requirements
### Requirement: Config directory resolution precedence
The CLI MUST resolve the configuration directory with the following precedence: CLI `--config-dir` override, `LLMAN_CONFIG_DIR` environment variable, then the ProjectDirs default for "llman".

#### Scenario: CLI override provided
- **WHEN** the user runs a command with `--config-dir`
- **THEN** the resolved config directory is the CLI value and `LLMAN_CONFIG_DIR` is set for subcommands

#### Scenario: Env override provided
- **WHEN** `LLMAN_CONFIG_DIR` is set and no CLI override is provided
- **THEN** the resolved config directory is the env value

#### Scenario: Default fallback
- **WHEN** no CLI or env override is provided
- **THEN** the resolved config directory is the ProjectDirs config path and resolution does not create directories

### Requirement: Invalid config directory errors
The resolver MUST return an error for invalid CLI/env config paths and MUST NOT create directories as part of resolution.

#### Scenario: Invalid CLI path
- **WHEN** `--config-dir` cannot be parsed as a valid path
- **THEN** the command returns an error that is surfaced by the CLI entrypoint

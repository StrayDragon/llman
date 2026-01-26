# config-paths Specification

## Purpose
Define how llman resolves the configuration directory and enforces safe defaults.
## Requirements
### Requirement: Config directory resolution precedence
The CLI MUST resolve the configuration directory with the following precedence: CLI `--config-dir` override, `LLMAN_CONFIG_DIR` environment variable, then the ProjectDirs default for "llman". The resolved value MUST be assigned to `LLMAN_CONFIG_DIR` for all subcommands, and resolution MUST NOT create directories.

#### Scenario: CLI override provided
- **WHEN** the user runs a command with `--config-dir`
- **THEN** the resolved config directory is the CLI value and `LLMAN_CONFIG_DIR` is set for subcommands

#### Scenario: Env override provided
- **WHEN** `LLMAN_CONFIG_DIR` is set and no CLI override is provided
- **THEN** the resolved config directory is the env value

#### Scenario: Default fallback
- **WHEN** no CLI or env override is provided
- **THEN** the resolved config directory is the ProjectDirs config path and resolution does not create directories

#### Scenario: Env propagation
- **WHEN** the CLI resolves a config directory
- **THEN** `LLMAN_CONFIG_DIR` is set to the resolved value for the process

### Requirement: Dev project guard requires explicit config dir
When running inside the llman development repository and neither CLI nor env override is provided, the CLI MUST return an error instructing the user to set `--config-dir` or `LLMAN_CONFIG_DIR`.

#### Scenario: Dev project without overrides
- **WHEN** the current directory contains a `Cargo.toml` with package name `llman` and no overrides are provided
- **THEN** the command fails with a config-dir-required error message

### Requirement: Invalid config directory errors
The resolver MUST return an error for empty or whitespace CLI/env config paths and MUST NOT create directories as part of resolution. Directory creation happens only when constructing a `Config` instance for prompt storage.

#### Scenario: Invalid CLI path
- **WHEN** `--config-dir` cannot be parsed as a valid path
- **THEN** the command returns an error that is surfaced by the CLI entrypoint

#### Scenario: Invalid env path
- **WHEN** `LLMAN_CONFIG_DIR` is set to an empty or whitespace value
- **THEN** the command returns an error that is surfaced by the CLI entrypoint

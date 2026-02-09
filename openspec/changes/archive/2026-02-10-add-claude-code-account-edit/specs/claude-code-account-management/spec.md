# claude-code-account-management Specification

## Purpose
Provide a first-class `account edit` entrypoint for Claude Code so users can quickly edit `claude-code.toml` using their configured editor, aligned with the existing Codex account edit experience.

## ADDED Requirements

### Requirement: Account edit command is available for Claude Code
The CLI MUST provide `llman x claude-code account edit` to open the Claude Code configuration file for editing.

#### Scenario: Edit command exists
- **WHEN** the user runs `llman x claude-code account edit`
- **THEN** the command attempts to open the Claude Code config file in an editor

### Requirement: Editor selection uses VISUAL/EDITOR with a safe fallback
When opening the configuration file for editing, the command MUST use `$VISUAL` if set and non-empty, otherwise `$EDITOR` if set and non-empty. If neither is set, it MUST fall back to `vi`.

#### Scenario: VISUAL takes precedence
- **WHEN** `$VISUAL` is set to `nvim` and `$EDITOR` is set to `code --wait`
- **THEN** the command uses `nvim` to open the config file

### Requirement: Editor command MUST support arguments
When opening the configuration file for editing, the command MUST support `$VISUAL` or `$EDITOR` containing arguments (for example `code --wait`). The implementation MUST execute the parsed command and MUST append the config file path as the last argument.

#### Scenario: Editor contains arguments
- **WHEN** `$EDITOR` is set to `code --wait` and the user runs `llman x claude-code account edit`
- **THEN** the executed command is `code --wait <claude-code.toml-path>`; if the editor exits non-zero the command returns an error

### Requirement: Config path resolution follows LLMAN_CONFIG_DIR
The command MUST open the configuration file at `<LLMAN_CONFIG_DIR>/claude-code.toml` (when `LLMAN_CONFIG_DIR` is set). Otherwise, it MUST use the platform default resolved by ProjectDirs for llman.

#### Scenario: LLMAN_CONFIG_DIR override
- **WHEN** `LLMAN_CONFIG_DIR` is set to `/tmp/llman-test-config` and the user runs `llman x claude-code account edit`
- **THEN** the command opens `/tmp/llman-test-config/claude-code.toml`

### Requirement: Missing config file is created with a minimal valid template
If the config file does not exist, the command MUST create the parent directory and MUST write a default template that is parseable by `Config::load()` (at minimum, it MUST include a `[groups]` table). The command MUST then open the created file in the editor.

#### Scenario: First-time edit creates template
- **WHEN** `<config-dir>/claude-code.toml` does not exist and the user runs `llman x claude-code account edit`
- **THEN** the command creates the directory, writes a minimal template, and launches the editor with that path

### Requirement: Editor exit status is enforced
If the editor process exits with a non-zero status, the command MUST fail with a non-zero exit and MUST surface a user-facing error message.

#### Scenario: Editor returns failure
- **WHEN** the selected editor exits with status 2
- **THEN** `llman x claude-code account edit` returns an error indicating the editor exit status

### Requirement: `x cc` alias supports account edit
Because `claude-code` is available as `cc`, the CLI MUST support `llman x cc account edit` with the same behavior as `llman x claude-code account edit`.

#### Scenario: Alias path works
- **WHEN** the user runs `llman x cc account edit`
- **THEN** the command behavior is identical to `llman x claude-code account edit`

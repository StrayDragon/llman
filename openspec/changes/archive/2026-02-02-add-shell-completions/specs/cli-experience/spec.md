## ADDED Requirements
### Requirement: Shell completion script generation
The CLI MUST provide `llman self completion --shell <shell>` to emit a completion script to stdout for bash, zsh, fish, powershell, and elvish.

#### Scenario: Generate bash completion
- **WHEN** a user runs `llman self completion --shell bash`
- **THEN** the completion script is written to stdout and the command exits successfully

### Requirement: Install completion snippet into shell rc
The CLI MUST provide `--install` to update the appropriate shell rc/profile file with a marked completion block for the selected shell.

#### Scenario: Install completion snippet
- **WHEN** a user runs `llman self completion --shell bash --install`
- **THEN** a marked completion block is added or updated in the bash rc/profile file and the command exits successfully

### Requirement: Install output is copyable
The `--install` command MUST print the exact snippet it applied so it can be copy/pasted.

#### Scenario: Install output is printed
- **WHEN** a completion snippet is installed
- **THEN** the snippet is written to stdout

### Requirement: Explicit confirmation for rc writes
The completion command MUST prompt for confirmation before modifying shell rc/profile files and MUST not write when confirmation is denied.

#### Scenario: Completion output has no side effects
- **WHEN** a user declines the confirmation prompt
- **THEN** no shell rc/profile files are modified

### Requirement: Idempotent install block
The completion install MUST not create duplicate blocks and MUST update an existing block in place when present.

#### Scenario: Existing block
- **WHEN** the shell rc/profile file already contains a marked llman completion block
- **THEN** the block is replaced with the latest snippet and no duplicate block is added

### Requirement: Unsupported shell handling
The CLI MUST reject unsupported shell values with a non-zero exit code and show the supported shell list.

#### Scenario: Unsupported shell value
- **WHEN** a user passes `--shell tcsh`
- **THEN** the CLI reports the supported shells and exits with failure

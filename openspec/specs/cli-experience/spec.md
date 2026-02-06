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
completion install MUST 在修改 shell rc/profile 文件前提示确认，且当用户拒绝时 MUST 不写入。若处于非交互环境，命令 MUST 拒绝写入并返回错误，除非显式提供 `--yes`；当提供 `--yes` 时 MUST 直接执行写入且不出现交互提示。

#### Scenario: 拒绝确认不会产生副作用
- **WHEN** 用户拒绝确认提示
- **THEN** 不修改任何 rc/profile 文件

#### Scenario: 非交互 install 未提供 --yes
- **WHEN** 命令在非交互环境运行，且包含 `--install` 但未提供 `--yes`
- **THEN** 命令以非零退出，且不修改任何 rc/profile 文件

#### Scenario: 非交互 install 提供 --yes
- **WHEN** 命令在非交互环境运行，且包含 `--install --yes`
- **THEN** completion block 被安装/更新，且命令成功退出

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

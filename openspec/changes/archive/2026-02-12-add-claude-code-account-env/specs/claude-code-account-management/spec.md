# claude-code-account-management Specification (Change: add-claude-code-account-env)

## ADDED Requirements

### Requirement: `account env` command emits env injection statements for a group
The CLI MUST provide `llman x claude-code account env <GROUP>` to emit shell-consumable env injection statements for the named configuration group.

The `<GROUP>` name MUST correspond to the same group name used by `llman x claude-code run --group <GROUP>`.

The CLI MUST also accept the alias path `llman x cc account env <GROUP>` with identical behavior.

The command MUST write only shell/PowerShell-consumable content to stdout:
- Comment lines beginning with `#` (usage hints)
- Injection statements for the selected group

It MUST NOT print any other informational text to stdout.

#### Scenario: Non-Windows emits POSIX export statements
- **WHEN** the user runs `llman x claude-code account env minimax` on a non-Windows platform and group `minimax` contains `FOO=bar`
- **THEN** stdout contains a line `export FOO='bar'`

#### Scenario: Windows emits PowerShell env statements
- **WHEN** the user runs `llman x claude-code account env minimax` on Windows and group `minimax` contains `FOO=bar`
- **THEN** stdout contains a line `$env:FOO='bar'`

### Requirement: Output is deterministic and safely quoted
The command MUST emit one injection statement per key/value pair in the selected group.

The command MUST sort keys in ascending lexicographic order before emitting statements.

Each value MUST be single-quoted to prevent interpolation:
- For POSIX output, embedded single quotes MUST be escaped so the resulting statement evaluates to the original value.
- For PowerShell output, embedded single quotes MUST be escaped using PowerShell single-quote escaping rules.

Each key MUST be validated as a safe environment variable name matching `^[A-Za-z_][A-Za-z0-9_]*$`. If any key is invalid, the command MUST fail and MUST NOT emit any injection statements.

#### Scenario: Keys are sorted for stable output
- **WHEN** group `g` contains keys `B=2` and `A=1`
- **THEN** stdout lines are emitted in the order `A` then `B`

#### Scenario: Invalid key fails without emitting statements
- **WHEN** group `g` contains a key `BAD-KEY=1`
- **THEN** the command exits non-zero and stdout contains no injection statements

### Requirement: Missing config or group is an error
If the Claude Code config contains no groups, the command MUST exit non-zero and MUST instruct the user to create or import a group.

If `<GROUP>` does not exist, the command MUST exit non-zero and MUST indicate the group was not found.

#### Scenario: Group not found
- **WHEN** the user runs `llman x claude-code account env does-not-exist`
- **THEN** the command exits non-zero and reports that the group does not exist

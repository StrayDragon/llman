# agent-tools-usage-stats Specification

## ADDED Requirements

### Requirement: Table output uses a real table renderer (no tabs)
When `--format table` is selected, the command MUST render a visually aligned table and MUST NOT rely on literal tab characters (`\\t`) for alignment.

Cell content shown in the table MUST be sanitized to avoid breaking the layout (at minimum, replacing `\\t` and newlines with spaces).

#### Scenario: Table output contains no tabs
- **WHEN** the user runs `llman x codex stats --format table`
- **THEN** stdout contains no `\\t` characters

### Requirement: Color policy is auto by default and can be overridden
The stats command MUST support `--color auto|always|never` for `--format table`.

The default MUST be `auto`.

Behavior:
- `auto`: enable ANSI colors only when stdout is a TTY and `NO_COLOR` is not set
- `always`: enable ANSI colors even when stdout is not a TTY
- `never`: disable ANSI colors

JSON output MUST NOT include ANSI escape sequences regardless of `--color`.

#### Scenario: NO_COLOR disables ANSI in auto mode
- **WHEN** the user runs `NO_COLOR=1 llman x cursor stats --format table --color auto`
- **THEN** stdout contains no ANSI escape sequences

#### Scenario: Non-TTY disables ANSI in auto mode
- **WHEN** the user runs `llman x claude-code stats --format table --color auto > out.txt`
- **THEN** `out.txt` contains no ANSI escape sequences

#### Scenario: JSON output is never colored
- **WHEN** the user runs `llman x codex stats --format json --color always`
- **THEN** stdout contains no ANSI escape sequences


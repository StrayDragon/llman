# tool-rm-useless-dirs Specification

## Purpose
TBD - created by archiving change update-tool-rm-useless-dirs. Update Purpose after archive.
## Requirements
### Requirement: Primary command name and alias
The CLI MUST expose `llman tool rm-useless-dirs` as the primary cleanup subcommand. It MUST accept `rm-empty-dirs` as a deprecated alias that triggers the same behavior and emits a deprecation warning.

#### Scenario: Run new command
- **WHEN** the user runs `llman tool rm-useless-dirs`
- **THEN** the cleanup behavior executes with the same option set as before.

#### Scenario: Run legacy alias
- **WHEN** the user runs `llman tool rm-empty-dirs`
- **THEN** the cleanup behavior executes and a deprecation warning referencing `rm-useless-dirs` is printed.

### Requirement: Protected directories
The tool MUST treat protected directory names as untouchable: it MUST NOT delete them and MUST NOT traverse into them, even when `--prune-ignored` is enabled.

The default protected list MUST include the following basenames:
- `.git`, `.hg`, `.svn`, `.bzr`
- `.idea`, `.vscode`
- `node_modules`, `.yarn`, `.pnpm-store`, `.pnpm`, `.npm`, `.cargo`
- `.venv`, `venv`, `.tox`, `.nox`, `__pypackages__`
- `target`
- `vendor`

#### Scenario: node_modules ignored
- **WHEN** `node_modules/` is ignored via .gitignore and the run enables `--prune-ignored` with `-y`
- **THEN** `node_modules` and its contents remain intact.

#### Scenario: protected directory is empty
- **WHEN** a protected directory exists and is empty
- **THEN** it is still preserved and not removed.

### Requirement: Useless directory allowlist
The tool MUST remove directories matching the useless allowlist even when non-empty. The default useless list MUST include `__pycache__`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.basedpyright`, `.pytype`, `.pyre`, `.ty`, `.ty_cache`, and `.ty-cache`.

#### Scenario: Remove __pycache__
- **WHEN** the target contains `a/__pycache__/b.pyc` and the run is live (`-y`)
- **THEN** `a/__pycache__` is removed.

#### Scenario: Remove .pytest_cache
- **WHEN** the target contains `.pytest_cache` and the run is live (`-y`)
- **THEN** `.pytest_cache` is removed.

### Requirement: Configurable protected/useless lists
The tool MUST support configuration under `tools.rm-useless-dirs` with separate `protected` and `useless` sections. Each section MUST support `mode` and `names`:
- `mode: extend` MUST combine the default list with the configured names.
- `mode: override` MUST replace the default list with the configured names.
- If the section is missing, defaults apply.

#### Scenario: Extend protected list
- **WHEN** config sets `protected.mode=extend` with `names: [".idea"]`
- **THEN** the protected list includes both defaults and `.idea`.

#### Scenario: Override protected list
- **WHEN** config sets `protected.mode=override` with `names: []`
- **THEN** no default protected names are applied.

### Requirement: Legacy config keys are rejected
The tool MUST NOT accept legacy config keys for this tool. If legacy keys such as `tools.rm-empty-dirs` are present, configuration loading MUST fail with a clear error.

#### Scenario: Legacy key present
- **WHEN** config contains `tools.rm-empty-dirs`
- **THEN** loading fails and reports the legacy key as unsupported.

### Requirement: 默认 gitignore 必须相对扫描目标解析
当提供扫描目标路径时，工具 MUST 将默认 `.gitignore` 解析为 `<target>/.gitignore`（当其存在且为文件）。工具 MUST NOT 对其它 target 隐式使用当前工作目录的 `.gitignore`。

#### Scenario: 非 CWD target 使用自己的 gitignore
- **WHEN** 用户运行 `llman tool rm-useless-dirs /tmp/project` 且未传 `--gitignore`
- **THEN** 若 `/tmp/project/.gitignore` 存在，则工具使用它

### Requirement: protected 名称在整棵路径树中生效
protected 目录 basenames MUST 在整个扫描树中生效。工具 MUST NOT 遍历进入任何路径中包含 protected 组件的目录。

#### Scenario: 扫描遇到 protected 组件
- **WHEN** 扫描遇到 `some/.git/objects`
- **THEN** 工具不进入 `some/.git` 子树，且不会删除其下任何内容

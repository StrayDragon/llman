# Capability: codex-agents-management

## ADDED Requirements

### Requirement: 提供 `llman x codex agents` 命令组
系统 MUST 提供 `llman x codex agents` 命令组以管理 Codex 的 custom agents 配置，并包含以下子命令：

- `import`：从目标 Codex agents 目录导入到 llman 托管目录
- `sync`：从 llman 托管目录同步到目标 Codex agents 目录
- `inject`：将 llman prompts（codex 模板）注入到托管的 agent TOML 的 `developer_instructions`
- `status`：展示托管目录与目标目录的差异与同步/注入可行性（只读）

#### Scenario: 查看帮助
- **WHEN** 用户运行 `llman x codex agents --help`
- **THEN** 帮助信息中包含 `import` / `sync` / `inject` / `status` 子命令说明

### Requirement: 提供 `status` 只读检查
`llman x codex agents status` MUST 以只读方式检查托管目录与目标目录的当前状态，并输出至少包含：

- 托管目录路径与目标目录路径
- 托管目录内的 `*.toml` 列表
- 对每个托管文件，目标侧是否存在同名文件，以及是否为“指向托管文件”的正确 symlink（或在 copy 模式下是否一致）
- 对每个托管文件，是否存在可注入的 `developer_instructions` 字段（用于提示 inject 是否会跳过）

`status` MUST NOT 写入任何文件（包括备份文件）。

#### Scenario: status 不落盘
- **WHEN** 用户运行 `llman x codex agents status`
- **THEN** 系统不写入任何文件且退出码为 0

### Requirement: 支持 `--dry-run` 展示计划但不落盘
`llman x codex agents import|sync|inject` MUST 支持 `--dry-run`，在该模式下系统 MUST：

- 计算将要执行的操作（例如：将复制哪些文件、将创建哪些 symlink、将生成哪些备份、将修改哪些文件）
- 输出该计划（plan）
- 不写入任何文件（包括备份文件）

#### Scenario: sync dry-run
- **WHEN** 用户运行 `llman x codex agents sync --dry-run`
- **THEN** 系统输出将执行的同步操作列表，但不修改目标目录

### Requirement: 非交互写操作需要显式确认（`--yes` / `--force`）
当 `import|sync|inject` 在非交互环境中运行且将产生任何文件写入时，系统 MUST 要求用户显式确认：

- 用户必须传入 `--yes` 或 `--force` 才能执行写操作
- 若未提供，则 MUST 失败并提示用户可使用 `--dry-run` 预览或使用 `--yes/--force` 确认执行

在交互环境中：
- 若将产生写操作且用户未提供 `--yes/--force`，系统 MUST 询问确认；用户取消则不落盘并正常退出。

#### Scenario: 非交互未确认则失败
- **WHEN** 在非交互环境中用户运行 `llman x codex agents sync` 且该操作会写入文件，但未提供 `--yes`/`--force`
- **THEN** 系统退出并提示需要 `--yes`/`--force` 或使用 `--dry-run`

### Requirement: 提供交互向导（inquire）
当用户在交互环境中运行 `llman x codex agents` 且未指定子命令时，系统 MUST 启动交互向导以收集参数并执行所选操作：

- 选择操作：`status` / `import` / `inject` / `sync`
- 根据所选操作，提供必要的选择：
  - import：从目标目录 `*.toml` 中 MultiSelect 选择要导入的条目（默认全选）
  - inject：从托管目录 `*.toml` 中选择目标文件（默认全选），并从 codex prompts 模板中选择要注入的模板（至少 1 个）
  - sync：从托管目录 `*.toml` 中选择要同步的条目（默认全选），并选择模式 `link` / `copy`
- 在执行任何写操作前，向导 MUST 展示计划并要求确认（等价于 `--dry-run` 的展示 + 确认执行）

### Requirement: llman 托管目录为 source of truth
系统 MUST 在 llman 配置目录下托管 Codex agents TOML，默认托管目录 MUST 为：

- `$LLMAN_CONFIG_DIR/codex/agents/`

托管目录中的 `*.toml` 文件 MUST 被视为同步源（source of truth）。

#### Scenario: 默认托管目录
- **WHEN** 用户运行 `llman x codex agents sync` 且未指定 `--managed-dir`
- **THEN** 系统从 `$LLMAN_CONFIG_DIR/codex/agents/` 读取 `*.toml` 并执行同步

### Requirement: 目标 Codex agents 目录可解析且可覆盖
系统 MUST 支持将同步/导入目标指向某个 Codex agents 目录，并按以下优先级解析目标目录：

1) `--agents-dir <path>`（直接指定 agents 目录）
2) `--codex-home <path>`（使用 `<path>/agents`）
3) 环境变量 `CODEX_HOME`（使用 `$CODEX_HOME/agents`）
4) 默认 `~/.codex/agents`

#### Scenario: 使用 `--agents-dir`
- **WHEN** 用户运行 `llman x codex agents sync --agents-dir /tmp/codex/agents`
- **THEN** 系统将输出同步到 `/tmp/codex/agents`

### Requirement: import 将目标 `*.toml` 纳入托管目录
`llman x codex agents import` MUST 将目标 agents 目录中的 `*.toml` 导入到托管目录中：

- 默认导入全部 `*.toml`
- 支持 `--only <name>`（可重复）仅导入 `<name>.toml`
- MUST NOT 删除托管目录中其它非本次导入范围的文件

当托管目录存在同名文件且内容/来源不确定时，系统 MUST 先备份再覆盖导入。

#### Scenario: 导入全部文件
- **WHEN** 目标 agents 目录包含 `a.toml` 与 `b.toml`，用户运行 `llman x codex agents import`
- **THEN** 托管目录生成/更新 `a.toml` 与 `b.toml`

#### Scenario: 仅导入指定文件
- **WHEN** 目标 agents 目录包含 `a.toml` 与 `b.toml`，用户运行 `llman x codex agents import --only a`
- **THEN** 托管目录仅生成/更新 `a.toml`，不导入 `b.toml`

### Requirement: sync 默认逐文件软链接同步
`llman x codex agents sync` MUST 将托管目录中的 `*.toml` 同步到目标 agents 目录中，默认模式 MUST 为逐文件软链接（symlink）：

- 对每个托管文件 `<managed>/<name>.toml`，目标 MUST 产生 `<target>/<name>.toml`
- 若目标文件已存在且不是期望的链接，系统 MUST 先备份再替换为期望链接
- 系统 MUST NOT 影响目标目录中不在本次同步范围内的其它文件

#### Scenario: 创建 symlink
- **WHEN** 托管目录存在 `defaults.toml`，目标目录无该文件，用户运行 `llman x codex agents sync`
- **THEN** 目标目录出现 `defaults.toml` 且其为指向托管文件的 symlink

### Requirement: sync 支持复制覆盖模式
`llman x codex agents sync` MUST 支持 copy 模式，在该模式下同步行为改为复制文件内容到目标目录（而不是创建 symlink），且冲突处理仍为备份后覆盖。

#### Scenario: copy 同步
- **WHEN** 用户运行 `llman x codex agents sync --mode copy`
- **THEN** 目标目录中的 `*.toml` 为常规文件（非 symlink），内容与托管目录一致

### Requirement: 冲突时默认备份后覆盖
当 `import` 或 `sync` 需要覆盖已有文件时，系统 MUST 先在同目录生成备份文件，再执行覆盖。备份文件名 MUST 追加：

- `.llman.bak.<timestamp>`

#### Scenario: sync 覆盖产生备份
- **WHEN** 目标目录已存在普通文件 `a.toml`，且将被同步替换
- **THEN** 目标目录产生 `a.toml.llman.bak.<timestamp>`，并将 `a.toml` 更新为同步结果

### Requirement: inject 将 prompts 模板片段注入到 `developer_instructions`
`llman x codex agents inject` MUST 从 llman 的 codex prompts 模板中读取片段，并注入到托管的 agent TOML 的 `developer_instructions = \"\"\"...\"\"\"` 字符串中：

- 模板内容 MUST 以 marker 进行幂等更新：
  - `<!-- LLMAN-PROMPTS:START -->`
  - `<!-- LLMAN-PROMPTS:END -->`
- 多模板注入时，每个模板 MUST 以标题段落包裹：`## llman prompts: <name>`，并按用户指定顺序拼接
- 若 TOML 不包含 `developer_instructions` 字段，系统 MUST 跳过该文件并提示（不应导致整次操作失败）

#### Scenario: 注入新 marker 区块
- **WHEN** 托管的 `reviewer.toml` 含有 `developer_instructions = \"\"\"...\"\"\"` 且无 marker，用户运行 `llman x codex agents inject --template common.en`
- **THEN** `developer_instructions` 内包含 marker 区块与 `## llman prompts: common.en` 段落

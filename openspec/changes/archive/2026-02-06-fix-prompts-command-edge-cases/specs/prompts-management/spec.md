## ADDED Requirements
### Requirement: 模板列举与读取在扩展名维度一致
对每个 `--app`，`llman prompts list` MUST 只展示 `llman prompts gen` 能够以确定方式读取的模板。命令 MUST 忽略该 app 模板目录中不属于该 app 支持扩展名集合的文件（例如备份文件、不同扩展名的同名文件）。

#### Scenario: 模板目录包含混合扩展名
- **WHEN** 模板目录同时包含支持的模板文件与不相关文件（例如不同扩展名的备份）
- **THEN** `llman prompts list --app <app>` 仅展示可读取模板
- **AND** 对任意被展示的模板执行 `gen` 都能成功读取（不出现 “rule not found”）

### Requirement: 非交互删除必须显式确认
`llman prompts rm` MUST 支持 `--yes` 用于跳过确认提示。若处于非交互环境，命令 MUST 拒绝删除并返回错误，除非用户显式提供 `--yes`。

#### Scenario: 非交互 rm 未提供 --yes
- **WHEN** 终端不可交互且用户运行 `llman prompts rm --app cursor --name foo`
- **THEN** 命令返回非零错误并提示需要传 `--yes`

#### Scenario: 非交互 rm 提供 --yes
- **WHEN** 终端不可交互且用户运行 `llman prompts rm --app cursor --name foo --yes`
- **THEN** 模板被删除且不会出现交互提示

### Requirement: Claude memory 注入在读取失败时不得静默覆盖
当向 `CLAUDE.md` 注入托管块时，如果目标文件已存在但无法作为 UTF-8 文本读取，命令 MUST 直接失败并 MUST NOT 对该路径进行写入。

#### Scenario: 既有 CLAUDE.md 不可读
- **WHEN** `<repo_root>/CLAUDE.md` 存在但无法作为 UTF-8 读取
- **THEN** `llman prompts gen --app claude-code --scope project --template <name>` 返回错误且文件未被修改

### Requirement: Project scope 必须通过 repo root 发现解析
对于 project-scope 目标路径，命令 MUST 通过向上查找父目录定位 git repo root，并 MUST 将 repo 内任意子目录视为有效 project 上下文。

#### Scenario: 在 repo 子目录中运行
- **WHEN** 用户在 repo 子目录中运行 `llman prompts gen --app codex --scope project --template <name>`
- **THEN** 输出被写入 `<repo_root>/.codex/prompts/`

### Requirement: Project scope 在无 git root 时必须显式 force
当用户请求 project-scope 但命令无法发现 git repo root 时，实现 MUST 默认拒绝写入并返回错误，除非用户显式确认/显式提供 `--force` 以允许将当前工作目录视为 project root。

#### Scenario: 非交互无 git root 且未提供 --force
- **WHEN** 终端不可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --app codex --template <name>`
- **THEN** 命令返回非零错误并提示需要 `--force`，且不会写入任何文件

#### Scenario: 非交互无 git root 但提供 --force
- **WHEN** 终端不可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --force --app codex --template <name>`
- **THEN** 命令将 `cwd` 视为 project root 并写入到 `<cwd>/.codex/prompts/`

#### Scenario: 交互无 git root 且用户拒绝强制执行
- **WHEN** 终端可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --app codex --template <name>` 并在提示中选择不强制执行
- **THEN** 命令安全退出且不会写入任何文件

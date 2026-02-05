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

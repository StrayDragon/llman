## ADDED Requirements

### Requirement: SDD 初始化脚手架
`llman sdd init` 命令 MUST 创建 `openspec/` 目录结构，包括 `openspec/AGENTS.md`、`openspec/project.md`、`openspec/specs/` 与 `openspec/changes/archive/`。当 `openspec/` 已存在时，命令 MUST 报错并且不修改任何文件。生成的 `openspec/AGENTS.md` MUST 包含 LLMAN-SDD 受管提示块。

#### Scenario: 初始化新项目
- **WHEN** 用户在不存在 `openspec/` 的目录执行 `llman sdd init`
- **THEN** 必要的目录结构与指令文件被创建

#### Scenario: 初始化时生成 SDD 提示块
- **WHEN** `llman sdd init` 生成 `openspec/AGENTS.md`
- **THEN** 文件中包含 `<!-- LLMAN-SDD:START -->` 与 `<!-- LLMAN-SDD:END -->` 包裹的提示块

#### Scenario: 已存在 openspec 目录
- **WHEN** 用户在已有 `openspec/` 的目录执行 `llman sdd init`
- **THEN** 命令返回错误且不做任何更改

### Requirement: SDD 指令与提示词刷新
`llman sdd update` MUST 刷新 `openspec/AGENTS.md` 与内置模板，同时 MUST 保持 `openspec/specs/**` 与 `openspec/changes/**` 不被修改。更新 `openspec/AGENTS.md` 时 MUST 仅替换 LLMAN-SDD 受管提示块，并保留受管块以外的用户内容。

#### Scenario: 更新指令文件
- **WHEN** 用户执行 `llman sdd update`
- **THEN** 指令/模板文件被刷新且现有 specs 与 changes 内容保持不变

#### Scenario: 保留用户自定义内容
- **WHEN** `openspec/AGENTS.md` 含有用户自定义内容且包含 LLMAN-SDD 受管块
- **THEN** update 仅替换受管块并保留其他内容

### Requirement: SDD 列表与查看
`llman sdd list` 默认 MUST 列出 `openspec/changes/` 下除 `archive` 外的变更 ID，提供 `--specs` 时 MUST 列出 `openspec/specs/` 下的 spec ID。`llman sdd show` MUST 输出指定 change/spec 的原始 markdown。`list` 与 `show` MUST 支持 `--json` 机器可读输出。

#### Scenario: 默认列出变更
- **WHEN** 用户执行 `llman sdd list`
- **THEN** 输出包含 `openspec/changes/` 下的变更目录（排除 `archive`）

#### Scenario: 列出 specs
- **WHEN** 用户执行 `llman sdd list --specs`
- **THEN** 输出包含 `openspec/specs/` 下的 spec 目录

#### Scenario: 查看变更
- **WHEN** 用户执行 `llman sdd show <change-id> --type change`
- **THEN** 输出 `openspec/changes/<change-id>/proposal.md` 的原始内容

#### Scenario: 查看 spec
- **WHEN** 用户执行 `llman sdd show <spec-id> --type spec`
- **THEN** 输出 `openspec/specs/<spec-id>/spec.md` 的原始内容

#### Scenario: JSON 输出
- **WHEN** 用户执行 `llman sdd list --json` 或 `llman sdd show <id> --json`
- **THEN** 输出为机器可读 JSON

### Requirement: SDD 校验
`llman sdd validate` MUST 校验 spec 与 delta 的格式（含 `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` 与 `#### Scenario:` 标题），并 MUST 支持 `--strict --no-interactive` 以及 `--json` 输出。

#### Scenario: 非法场景标题
- **WHEN** 某个 requirement 使用了不合法的场景标题（非 `#### Scenario:`）
- **THEN** 校验失败并报告具体文件

#### Scenario: 合法变更
- **WHEN** 变更包含完整且格式正确的 deltas 与必需文件
- **THEN** 校验成功并返回退出码 0

#### Scenario: 校验 JSON 输出
- **WHEN** 用户执行 `llman sdd validate <id> --json`
- **THEN** 输出为机器可读 JSON

### Requirement: SDD 归档流程
`llman sdd archive` MUST 将 delta 合并到 `openspec/specs` 并将变更目录移动到 `openspec/changes/archive/YYYY-MM-DD-<change-id>`。命令 MUST 支持 `--skip-specs` 以在不更新 specs 的情况下归档。

#### Scenario: 归档并更新 specs
- **WHEN** 用户执行 `llman sdd archive <change-id>`
- **THEN** specs 被 delta 更新且变更目录被移动到 archive

#### Scenario: 仅归档目录
- **WHEN** 用户执行 `llman sdd archive <change-id> --skip-specs`
- **THEN** 变更目录被移动到 archive 且 specs 不被修改

### Requirement: SDD 归档 dry-run
`llman sdd archive --dry-run` MUST 输出将要修改/移动的文件与目标路径，并 MUST 不进行任何文件写入。

#### Scenario: 归档 dry-run
- **WHEN** 用户执行 `llman sdd archive <change-id> --dry-run`
- **THEN** 输出预览信息且文件系统无任何改动

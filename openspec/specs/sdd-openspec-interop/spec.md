# sdd-openspec-interop Specification

## Purpose
TBD - created by archiving change add-sdd-openspec-import-export. Update Purpose after archive.
## Requirements
### Requirement: OpenSpec 风格导入命令
系统 MUST 提供 `llman sdd import --style openspec [path]`，用于将 `openspec/` 目录迁移为 `llmanspec/` 结构。  
`--style` MUST 为必填，且当前仅允许值 `openspec`。

#### Scenario: 导入命令参数有效
- **WHEN** 用户执行 `llman sdd import --style openspec`
- **THEN** 命令开始构建从 `openspec/` 到 `llmanspec/` 的迁移计划

#### Scenario: 导入 style 非法
- **WHEN** 用户执行 `llman sdd import --style unknown`
- **THEN** 命令返回非零并提示仅支持 `openspec`

### Requirement: OpenSpec 风格导出命令
系统 MUST 提供 `llman sdd export --style openspec [path]`，用于将 `llmanspec/` 目录迁移为 `openspec/` 结构。  
`--style` MUST 为必填，且当前仅允许值 `openspec`。

#### Scenario: 导出命令参数有效
- **WHEN** 用户执行 `llman sdd export --style openspec`
- **THEN** 命令开始构建从 `llmanspec/` 到 `openspec/` 的迁移计划

#### Scenario: 导出 style 非法
- **WHEN** 用户执行 `llman sdd export --style unknown`
- **THEN** 命令返回非零并提示仅支持 `openspec`

### Requirement: 默认演练与交互双确认
`import/export` MUST 默认先输出完整 dry-run 计划，再进入执行门禁。  
真正写入 MUST 仅在交互式终端中完成两次确认后执行。  
非交互环境 MUST 直接拒绝写入并返回非零退出。

#### Scenario: 交互环境双确认后执行
- **WHEN** 用户在交互终端执行 `llman sdd import --style openspec` 并依次通过两次确认
- **THEN** 命令执行迁移写入

#### Scenario: 非交互环境拒绝执行
- **WHEN** 用户在非交互环境执行 `llman sdd export --style openspec`
- **THEN** 命令输出迁移计划后返回非零
- **AND** 不修改任何文件

### Requirement: 完整迁移范围与冲突策略
迁移范围 MUST 包含 `specs`、active `changes`、`changes/archive`。  
目标路径如存在同名冲突，命令 MUST 失败并中止；实现 MUST NOT 覆盖也 MUST NOT 跳过冲突文件继续。  
当检测到非标准目录（例如 `openspec/explorations/`）时，命令 MUST 输出 warning，并将该目录按相对路径一并复制到目标侧。

#### Scenario: 目标冲突即失败
- **WHEN** 目标目录中已存在将写入的同名文件
- **THEN** 命令返回非零
- **AND** 不进行覆盖写入

#### Scenario: 检测到非标准目录
- **WHEN** 导入源包含 `openspec/explorations/`
- **THEN** 命令输出 warning，说明检测到非标准目录
- **AND** 命令在执行写入阶段复制该目录内容到目标侧对应路径

### Requirement: 迁移后旧目录删除确认
在迁移写入成功后，命令 MUST 在交互模式下提示用户是否删除旧迁移目录（源目录），并且默认选项 MUST 为“不删除”。  
如果用户未确认删除，系统 MUST 保留旧目录。  
在非交互模式下，系统 MUST NOT 删除旧目录。

#### Scenario: 交互模式默认不删除旧目录
- **WHEN** 迁移执行成功且系统进入删除确认提示
- **THEN** 默认选项为“不删除”
- **AND** 用户直接确认默认选项后，旧目录保持不变

#### Scenario: 用户确认删除旧目录
- **WHEN** 迁移执行成功且用户在交互提示中明确选择删除
- **THEN** 系统删除旧迁移目录

#### Scenario: 非交互模式不删除旧目录
- **WHEN** 用户在非交互模式下运行 `import/export` 命令
- **THEN** 系统不会执行旧目录删除操作

### Requirement: 元数据补齐与规范兼容
`export` 时系统 MUST 自动补齐 OpenSpec 侧元数据：  
- 若缺失则创建 `openspec/config.yaml`（至少包含 `schema: spec-driven`）  
- 为每个 active change 补齐 `.openspec.yaml`（包含 `schema` 与 `created`）  

`import` 时若主 spec 缺失 llman frontmatter，系统 MUST 补齐最小合法 frontmatter（`llman_spec_valid_scope`、`llman_spec_valid_commands`、`llman_spec_evidence`）。

#### Scenario: 导出自动补齐元数据
- **WHEN** 用户执行 `llman sdd export --style openspec` 且目标缺失 `openspec/config.yaml`
- **THEN** 命令在执行写入阶段创建 `openspec/config.yaml`

#### Scenario: 导入补齐 llman frontmatter
- **WHEN** 导入源 spec 缺失 `llman_spec_valid_scope`
- **THEN** 命令在执行写入阶段为目标 spec 补齐必需 frontmatter


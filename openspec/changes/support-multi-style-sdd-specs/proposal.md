## Why

当前 `llman sdd` 的 spec / delta spec 语义已经收敛到单一 canonical table/object ISON。这个方向在实现上简单，但也带来了新的约束：一旦团队希望在 SDD 工件层尝试更紧凑或更易扩展的结构化格式，就只能在仓库外手工转换，CLI 无法识别、校验、归档或统一改写这些格式。

我们现在需要把 SDD 的“规范承载格式”从单一格式扩展为“单项目单主风格”的实验性多风格机制，同时保持运行时边界严格可控：

- 一个项目内必须显式声明其主风格，避免同仓库混用多种语法导致解析、校验、归档结果不确定。
- 一旦项目声明了主风格，CLI 就只接受该风格；遇到其他风格或无法识别的内容必须直接报错，而不是猜测或隐式兼容。
- 如果项目尚未声明主风格，相关 `llman sdd` 命令必须拒绝继续，并要求用户先完成配置，而不是偷偷沿用默认值。
- 除了识别和写回之外，CLI 还需要提供显式的风格转换能力，允许用户在 `ISON / TOON / YAML` 三种风格之间做可审计、可验证的迁移。

这次变更的目标不是“宽松支持多格式”，而是建立一个严格、可迁移、可回滚的实验性多风格框架：项目级显式选择一种主风格，所有读写和验证都围绕该风格运行，跨风格切换只能通过明确的转换动作完成。

## What Changes

- 为 `llmanspec` 项目配置增加显式的 SDD spec 主风格设置，支持三种实验性风格：`ison`、`toon`、`yaml`。
- 将该主风格配置同时应用到主 specs 与 change delta specs：
  - `llmanspec/specs/<capability>/spec.md`
  - `llmanspec/changes/<change>/specs/<capability>/spec.md`
- 调整 `llman sdd` 的 spec 读取链路，使 `show`、`list`、`validate`、`archive`、authoring helpers 等命令都按照项目配置的主风格解析文件。
- 新增严格配置门禁：
  - 如果项目未配置主风格，相关 SDD spec 命令必须失败，并明确提示用户先设置主风格。
  - 如果项目已配置主风格，但遇到不属于该风格的 spec / delta spec 文件，命令必须失败，并明确指出期望风格与实际内容不匹配。
- 引入三种风格之间的显式转换能力，覆盖主 spec 与 delta spec，并支持面向单文件或项目范围的转换工作流。
- 保持现有 SDD 语义层不变：无论底层使用 `ISON / TOON / YAML` 中哪一种承载格式，需求、场景、delta op、archive merge 的语义结果必须一致。
- 将多风格支持标记为实验性能力，并在错误提示、帮助文本与模板/脚手架中明确边界，避免用户误以为可以在同一项目中混用格式。

## Capabilities

### New Capabilities

- `sdd-multi-style-formats`: 定义 `ISON / TOON / YAML` 三种实验性 SDD spec/delta spec 风格、项目级主风格约束、严格识别策略与显式转换能力。

### Modified Capabilities

- `sdd-workflow`: 更新工作流约束，要求项目先声明主风格后才能运行相关 spec/delta 命令，并将风格错误视为硬失败。
- `sdd-ison-authoring`: 从“仅 canonical ISON authoring”扩展为“按项目主风格 authoring”，同时保留统一语义模型。
- `sdd-ison-pipeline`: 将当前 ISON-only 解析/写回链路提升为“多风格解析 + 统一语义归一化 + 按主风格序列化”的实验性管线。

## Impact

- **BREAKING**：未配置 SDD 主风格的项目，在运行依赖 spec / delta spec 的 `llman sdd` 命令时将直接失败，用户必须先完成配置。
- **BREAKING**：项目一旦声明主风格，仓库中的主 spec 与 change delta spec 都必须符合该风格；过去依赖单一 canonical ISON 的仓库，如果想改用 `TOON` 或 `YAML`，必须先执行显式转换。
- `validate`、`show`、`archive`、`spec` / `delta` authoring helpers、模板初始化与文档示例都将受影响，需要统一围绕“项目级主风格”更新行为与提示。
- 为避免隐式兼容扩大复杂度，本变更不允许“未配置时默认按 ISON 解析”，也不允许“已配置为 A 但自动读取 B 再写回 A”的宽松模式。
- 由于这是实验性能力，测试与验收需要重点覆盖：
  - 三种风格的主 spec 解析与校验
  - 三种风格的 delta spec 解析、归档合并与严格报错
  - 配置缺失、风格错误、转换前后语义一致性

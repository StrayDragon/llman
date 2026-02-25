## Why

随着 `llmanspec/changes/archive/` 长期累积，归档目录规模会快速增长，导致 coding agent 在检索上下文时频繁读取或改动旧归档内容，干扰当前变更实施与评审。与此同时，当前 llman SDD 缺少面向“未来分叉/延后需求”的标准化承载结构，skills 提示词也缺乏统一的低熵结构约束，造成执行风格不稳定、可验证性不足。

需要一次有边界的 SDD 升级：在不触碰 `openspec/` 既有流程的前提下，为 `llmanspec` 引入归档冻结能力、future-changes 结构化记录方式、以及可测试的结构化提示规范。

## What Changes

- 升级 `llman sdd archive` 为子命令组，保留 `llman sdd archive <change-id>` 兼容入口，并新增：
  - `llman sdd archive run <change-id>`
  - `llman sdd archive freeze`
  - `llman sdd archive thaw`
- 新增归档冻结机制（单一 7z 冷备归档文件）：
  - 将 `llmanspec/changes/archive/` 下日期归档目录持续写入同一个归档文件
  - 冻结目标文件固定为 `llmanspec/changes/archive/freezed_changes.7z.archived`
  - 采用原子写入与失败回滚约束，保证可恢复与不丢数据
- 新增每个 change 的 future 记录文件规范：`llmanspec/changes/<id>/future.md`（可选但受流程引导）
- 升级 llman SDD skills/spec-driven 模板为“结构化规则内化”风格：
  - 明确 Context/Goal/Constraints/Workflow/Decision Policy/Output Contract
  - 不直接要求使用外部技能，不暴露外部依赖
- 新增 `llman-sdd-specs-compact` skill（技能优先），并在 specs 中预留未来 CLI 入口（本次不实现 `sdd specs compact`）。

## Capabilities

### New Capabilities

- `sdd-archive-freeze`: 为 llman SDD 提供归档冻结与解冻能力，降低旧归档对 agent 的干扰。
- `sdd-future-changes`: 为单个 change 提供结构化 future 记录承载，支持延期项与分叉路线管理。
- `sdd-structured-skill-prompts`: 为 llman-sdd-* 与 spec-driven 模板提供结构化、低熵、可验证提示协议。
- `sdd-specs-compaction-guidance`: 提供 specs 压缩治理 skill 与后续 CLI 演进预留。

### Modified Capabilities

- `sdd-workflow`: 扩展 archive 命令面、技能模板规范、以及 change 文档组织约束。

## Impact

- 新增依赖：引入活跃维护的 `sevenz-rust2` 作为 7z 压缩/解压实现。
- 受影响代码：`src/sdd/command.rs`、`src/sdd/change/archive.rs`、`src/sdd/project/templates.rs`、`src/sdd/project/init.rs`、`src/sdd/shared/validate.rs`（如增加 future 轻量校验）。
- 新增或调整模板：`templates/sdd/{en,zh-Hans}/skills/*`、`templates/sdd/{en,zh-Hans}/spec-driven/*`、共享 region 文档。
- 新增测试覆盖：`tests/sdd_integration_tests.rs`（archive freeze/thaw 单文件冷备流程）、模板与 skills 生成回归、future 文件流程引导。
- 兼容性：保持 `llman sdd archive <change-id>` 现有行为；新增能力默认手动触发，避免破坏已有自动化脚本。

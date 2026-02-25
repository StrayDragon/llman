## Context

当前 `llman sdd archive` 只覆盖“delta 合并 + change 目录移动”，没有历史归档冻结能力。随着 `llmanspec/changes/archive/` 增长，agent 在检索时会频繁触达旧归档，干扰当前任务。现有 change 工件也缺少对“未来分叉/延期事项”的标准化承载。

我们需要在保持现有 CLI 可用性的前提下，扩展归档能力并升级 llman SDD skills 模板规范，同时保持改动边界清晰、可测试、可回归。

约束：
- 本次仅处理 `llmanspec`，不修改 `openspec` 运行路径。
- 保留 `llman sdd archive <change-id>` 兼容行为。
- 冻结使用单一归档文件 `freezed_changes.7z.archived`，不维护额外索引数据库。
- specs 压缩本次仅交付 skill 规范，CLI 仅预留。

## Goals / Non-Goals

**Goals:**
- 扩展 `llman sdd archive` 为可管理冻结生命周期的子命令组。
- 提供可恢复的归档冻结机制（单一冷备归档文件）。
- 引入 `future.md` 作为每个 change 的未来路线承载文件。
- 将结构化、低熵提示规则内化到 llman SDD skills/spec-driven 模板。
- 交付可验证、可测试的任务拆分与验收标准。

**Non-Goals:**
- 不对 `openspec/changes/archive` 提供同批冻结能力。
- 不在本次实现 `llman sdd specs compact` CLI。
- 不做自动触发冻结（阈值触发/归档后自动触发均不做）。

## Decisions

### 1. 命令面升级为 archive 子命令组，保持兼容入口
- 新增 `archive run/freeze/thaw`，并保留 `llman sdd archive <change-id>` 作为兼容快捷入口（内部路由至 `archive run`）。
- 原因：既能提供清晰操作语义，又避免破坏现有调用脚本。
- 备选方案：
  - 仅加 `archive --freeze` 等参数：语义混杂，可维护性差。
  - 新增 `sdd freeze` 顶级命令：学习成本高，与 archive 生命周期脱节。

### 2. 冻结介质采用单一 `7z` 冷备文件
- 冻结输出固定为：`llmanspec/changes/archive/freezed_changes.7z.archived`
- `archive freeze` 将本次候选归档目录合并写入同一文件。
- 不维护 `index.json`，以“冷备文件 + 解冻恢复”为主能力。
- 原因：满足“统一冷备文件 + 可持续追加”诉求，降低管理复杂度与元数据维护成本。
- 依赖选型：使用活跃维护的 `sevenz-rust2` crate（纯 Rust 7z 压缩/解压）。
- 追加语义实现：当冷备文件已存在时，执行“读旧归档 + 合并新候选 + 原子重写同一路径”，对用户保持“单文件持续追加”的行为。

### 3. 手动触发冻结，默认最小副作用
- 仅在用户显式执行 `llman sdd archive freeze` 时触发。
- 默认解冻到 `llmanspec/changes/archive/.thawed/`，避免旧归档立刻重新暴露给 agent。
- 原因：避免自动化策略误伤当前开发流程。

### 4. future-changes 采用每变更独立文件
- 标准路径：`llmanspec/changes/<change-id>/future.md`
- 标准章节：Deferred Items / Branch Options / Triggers to Reopen / Out of Scope。
- `new/ff/continue/explore` 模板中加入引导，不强制文件存在。
- 原因：上下文靠近变更，便于未来回溯与裁剪。

### 5. skills 模板采用“结构化规则内化”
- 在 `llman-sdd-*` 与 `spec-driven/*` 中统一结构层：Context/Goal/Constraints/Workflow/Decision Policy/Output Contract。
- 不要求使用 ISON 语法，也不直接引用外部技能。
- 原因：降低执行歧义并提升提示词一致性，同时避免外部依赖耦合。

### 6. specs 压缩采用“技能先行，CLI 预留”
- 新增 `llman-sdd-specs-compact` skill，定义压缩流程与验收。
- 在规范中预留 `llman sdd specs compact` 的未来接口，不在本次实现命令。
- 原因：先解决流程一致性，再做 CLI 产品化，降低一次性改动风险。

## Risks / Trade-offs

- [风险] 冻结/解冻涉及批量文件移动，失败可能造成目录不一致。
  -> [缓解] 采用临时文件 + 原子 rename；仅在冷备文件写入成功后删除源目录。

- [风险] 7z crate 未提供稳定的原地 append API，直接追加可能不可控。
  -> [缓解] 采用“合并后重写同一路径”的逻辑追加，并通过回归测试验证历史内容不丢失。

- [风险] 兼容入口与新子命令并存可能导致帮助文本复杂。
  -> [缓解] 在 `--help` 中明确“兼容入口”说明，示例统一指向子命令组。

- [风险] skills 模板结构化升级可能引起既有 prompt 输出风格变化。
  -> [缓解] 通过模板版本号与集成测试锁定关键段落和必需字段。

- [风险] future.md 引导可能被误解为强制校验。
  -> [缓解] 明确标注“可选文件”，验证只做存在性外的轻量结构检查（如启用）。

## Migration Plan

1. 先更新 OpenSpec artifacts（本 change 的 proposal/design/specs/tasks）。
2. 实现 `archive` 子命令组与兼容路由。
3. 实现 freeze/thaw（单文件 7z 冷备）与集成测试。
4. 更新 SDD 模板和 skills 生成清单，新增 specs-compact skill 模板。
5. 增补 `future.md` 相关模板与流程引导。
6. 运行回归：`just test`、`just check-sdd-templates`，并补 `sdd --help` 输出断言。

回滚策略：
- 若冻结能力出现问题，可先停用 `archive freeze/thaw` 路由，不影响既有 `archive <id>` 主流程。
- 模板变更可通过 `llman sdd update` 重新生成到稳定版本。

## Open Questions

- 无阻塞性开放问题；后续仅保留“`sdd specs compact` CLI 参数设计”作为下一阶段议题。

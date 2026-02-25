## Context

`llman` 仓库同时维护 `llmanspec/` 与 `openspec/` 语义，但目前缺少官方双向互转命令。  
用户已明确要求：
- 命令命名采用 `import/export`，不使用 `migrate --from/--to`
- `--style openspec` 必填且目前唯一支持值
- 命令默认安全演练，且只能在交互式双确认后执行写入
- 非交互环境禁止落盘
- 迁移范围必须完整覆盖 specs、active changes、archive changes

这意味着设计必须同时满足“可迁移”和“防误操作”两类目标。

## Goals / Non-Goals

**Goals:**
- 提供稳定、可预期的双向互转入口（`import/export`）。
- 用统一执行模型防止 agent/CLI 误写文件系统。
- 让迁移结果在两侧工具链上可直接消费（含必要元数据补齐）。
- 为后续 style 扩展预留接口，但不提前实现额外 style。

**Non-Goals:**
- 不支持除 `openspec` 之外的 style。
- 不提供 `migrate` 兼容别名。
- 不引入复杂的回滚事务引擎或跨进程锁。

## Decisions

### Decision 1: 命令接口收敛为 import/export
- 采用：
  - `llman sdd import --style openspec [path]`
  - `llman sdd export --style openspec [path]`
- 拒绝：
  - `migrate --from/--to`：语义不够直观且易与“版本迁移”混淆。

### Decision 2: 强制“计划先行”执行模型
- 所有互转命令都走固定阶段：
  1. 扫描源与目标并构建计划（只读）
  2. 输出 dry-run 计划
  3. 执行门禁（交互双确认 / 非交互拒绝）
  4. 执行写入
- 优点：即使用户误触发命令，也先看到变更计划而不是直接写盘。

### Decision 3: 交互双确认 + 非交互拒绝
- 交互环境：`Confirm` + 输入确认短语，两步都通过才写入。
- 非交互环境：返回非零，保证“只能演练不能执行”。
- 替代方案（`--yes` 或 `--apply`）被拒绝：与用户的“仅 interactive inquire 执行”要求冲突。

### Decision 4: 路径与冲突策略
- 冲突默认失败并中止；不覆盖、不跳过。
- 严格校验标识符与路径边界，禁止越界写入。
- 非标准目录纳入迁移复制范围，但必须输出显式 warning，提示其不属于标准规范目录。

### Decision 5: 迁移后旧目录处理
- 成功迁移并写入后，在交互模式中提示是否删除旧迁移目录（源目录），默认选择必须是“否”。
- 非交互模式不提供删除动作，保持旧目录不变。

### Decision 6: 元数据兼容策略
- export 方向自动补齐 OpenSpec 关键元数据：
  - `openspec/config.yaml`（`schema: spec-driven`）
  - active change `.openspec.yaml`（`schema`, `created`）
- import 方向缺失 llman spec frontmatter 时自动补齐最小合法字段。

## Risks / Trade-offs

- [风险] 非交互环境无法直接执行真实迁移，CI 无法“一键迁移落盘”。
  - 缓解：该约束是明确安全需求，保留 dry-run 输出用于审阅与人工执行。
- [风险] 非标准目录直接复制可能引入额外历史内容，导致目标目录膨胀。
  - 缓解：输出 warning 并在报告中单独列出复制的非标准目录。
- [风险] 用户误删旧目录会造成回看不便。
  - 缓解：交互删除默认“否”，且仅在迁移成功后提示删除。
- [风险] 自动补齐元数据可能与个别团队自定义实践不一致。
  - 缓解：采用最小可用默认值，后续可再扩展可配置策略。

## Migration Plan

1. CLI 层新增 `import/export` 命令定义与参数约束（`--style` 必填）。
2. 新增 `interop` 执行模块，统一实现计划构建、校验、门禁与写入。
3. 落地双向映射逻辑与元数据补齐逻辑。
4. 添加 i18n 文案与错误提示。
5. 新增/更新集成测试覆盖：
   - 非交互拒绝执行
   - 双确认流程
   - 冲突失败
   - 非标准目录 warning 与复制
   - 旧迁移目录删除确认（默认否）
   - 完整范围迁移与元数据补齐
6. 更新 `sdd-workflow` 主规范与文档，明确命令面与安全模型。

## Open Questions

- 当前无阻塞性开放问题；后续可评估是否需要支持 additional styles（例如自定义 schema 互转）。

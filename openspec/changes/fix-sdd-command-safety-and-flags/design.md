## Context
`llman sdd` 在 repo 内维护 `llmanspec/` 树，并基于用户输入的标识符执行文件系统操作（写入、移动、归档、校验）。当前若将标识符直接当作路径片段使用，存在路径穿越与越界写入风险；同时部分 CLI 参数边界与批量校验性能存在改进空间。

## Goals / Non-Goals
- Goals:
  - 阻断路径穿越与非预期目录移动/写入。
  - 明确 `list` 参数语义与冲突策略（与 `sdd-workflow` 规范对齐）。
  - `update-skills` 的 `--path` 覆盖在 multi-tool 场景下保持确定性与安全（避免输出互相覆盖）。
  - 在不改变校验语义的前提下，优化批量校验的 git 调用开销。
- Non-Goals:
  - 不新增 SDD 子命令。
  - 不做大规模 parser/格式重构。

## Decisions
- Decision: 引入共享的“标识符校验”辅助函数，用于所有 change/spec ID → path join 的位置。
  - Rationale: 集中化安全规则，避免不同子命令各自实现不一致。
- Decision: `list` 的 `--specs` 与 `--changes` 互斥，冲突时报错。
  - Rationale: 避免用户误解“同时指定两种模式”的含义。
- Decision: `update-skills` 在 multi-tool 且传入单个 `--path` 覆盖时，默认失败并给出明确提示（按 tool 分开执行）。
  - Rationale: 最安全默认；避免 silent clobber。

## Risks / Trade-offs
- 收紧 ID 校验可能会拒绝此前“非常规但能跑”的 ID；缓解：规则保持最小（拒绝分隔符与 `..`），并在错误提示中给出合法示例。
- 批量校验性能优化必须不改变 strictness/结果；缓解：只缓存共享 git 结果，不改变判定逻辑。

## Migration Plan
- 无数据迁移。需在变更说明中记录行为变化：
  - `list` 同时传 `--specs --changes` 由隐式优先变为显式错误。
  - `update-skills` multi-tool + `--path` 由潜在覆盖变为显式拒绝。

## Open Questions
- 无（本变更避免引入新的语义分歧，保持 `validate` 对缺失 delta 的错误判定不变）。

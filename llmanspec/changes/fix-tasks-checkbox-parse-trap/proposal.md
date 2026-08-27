---
depends_on: []
---

实施候选：快速路径优先（内部解析器细节，无行为合约文本牵连）；若实现中发现某
capability 已隐含 MUST 语义，再升级走 propose。

## Why

`src/sdd/shared/tasks.rs::is_checkbox_line` 把任何 `- [` 前缀行当勾选框，
内容非空即归类 Pending；而 r101 允许的 `[blocked-by]` 依赖标记若按直觉写在
独立行首（如 `- [blocked-by: T1]`），会被当成未完成任务，在
`validate --strict` 下直接 ERROR——提案阶段的干净工件反而过不了门禁。
实际触发于 toon-longtail-consistency-purge 的 propose（b306322 被迫返工重写）。

归档惯例的绕法是把依赖写成任务行内尾缀（`- [x] task-N: … [blocked-by: none]`），
但该惯例没有任何文档约束，且语义上依赖标记不是任务本身。

## What Changes

- 解析收窄：仅 `[ ]` / `[x]` / `[X]`（含前导空格变体）识别为勾选框；
  其余方括号内容的行（如 `[blocked-by: …]`、自定义 tag）视为普通文本条目，
  不进入 completion 统计。
- 行首合法勾选框的提取逻辑（extract_task_text）同步适配，不改变既有
  `- [x]/- [ ]` 用例行为。
- validation-hints 或 templates 中补一句推荐写法：依赖标注建议放在任务行尾。

## Non-goals

- 不引入 markdown 任务列表标准以外的语法支持。
- 不改变 list/show 对 tasks 完成度的展示口径（completion_ratio 语义不变）。

## Verification Sketch

- shared/tasks.rs 单测：`- [blocked-by: x]` 不计数；`[x]`/`[X]`/`[ ]` 行为不变。
- 构造含独立行 `[blocked-by]` 风格 tasks.md 的临时项目跑 `validate --strict`
  应通过；现有全部 fixture 与 smoke 回归绿。

## Open Questions

- Blocked-by 若用户写成因笔误悬空（引用不存在的 task id），要不要给 WARNING？
  可与 check_dag_cycles 现有校验联动评估。

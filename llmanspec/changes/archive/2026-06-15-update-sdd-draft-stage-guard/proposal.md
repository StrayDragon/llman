---
depends_on: []
blocks: []
---

# 将 draft 提案提升为一等公民阶段

## Why

当前 `llman sdd` 已经能检测变更的完整度阶段(`draft` / `specified` / `designed` / `full`，见 `sdd-workflow` r43)，但 **draft（仅有 `proposal.md`）尚未作为一等公民被全链路处理**，存在三处缺口：

1. **实现侧无守卫**：`llman-sdd-apply` / `llman-sdd-verify` skill（r33 / r35）不感知阶段，agent 会对一个只有 `proposal.md` 的 draft 提案直接尝试"实现/验证"，产出无 spec、无 tasks、无法 verify 的半成品。
2. **`show` 不暴露阶段**：阶段信息只出现在 `validate` / `list`（r43 / r46），`llman sdd show <id> --json` 的 change 输出无 `stage` 字段，守卫逻辑只能靠启发式猜，无法基于权威数据。
3. **非 strict 下 draft 的阶段提示不可见**：实测 `llman sdd validate <draft> --no-interactive` 输出 `valid` 后直接返回，r45 要求的 INFO 级阶段提示被吞掉，用户在非 strict 下完全看不到"draft，请先继续完善"的引导。

draft 是变更的正常过渡态（`propose` 之后、`continue` 长大之前的起点），应被**保护**而非被越过实现。

## What Changes

- **守卫 + 引导完善**（核心方向）：`apply` / `verify` 在变更未达 `full` 阶段时拒绝执行实现/验证，引导用户走 `continue` 把 draft 长大成 `full` 后再实现。draft 是"还没准备好被实现"的过渡态，不是可绕过的草稿。
- **`show --json` 暴露权威阶段**：change 的 JSON 输出新增 `stage`、`artifacts`、`readyToImplement` 字段，作为守卫的唯一数据源。
- **`show` 文本模式展示阶段**：change 摘要中显示当前 stage。
- **修非 strict 下 draft 的 INFO 偏差**：让 r45 要求的阶段 INFO 在非 strict 模式下也能被用户看到。
- **`continue` 反向感知 draft**：draft 阶段显式提示"这是 draft 提案，需先补 specs → design → tasks 长大成 full"。

## Capabilities

- sdd-workflow（MODIFY r33 apply / r35 verify / r45 分级消息 / r46 show stage；ADD 新增阶段守卫场景与 readyToImplement 语义）

## Impact

- **CLI 行为**：`llman sdd show <id>` 的 JSON/text 输出结构扩展（新增字段，向后兼容）；非 strict 下 valid 的 change 会多打印一条 INFO 级阶段提示（新输出，不影响 exit code）。
- **Skill 模板**：`templates/sdd/{locale}/skills/llman-sdd-apply.md`、`llman-sdd-verify.md`、`llman-sdd-continue.md` 的前置检查逻辑更新；下游 `llman sdd update-skills` 重新生成的产物需相应刷新。
- **测试**：新增针对 stage 暴露与守卫场景的集成测试；补 r45 非 strict INFO 回归。
- **向后兼容**：不改变 `propose` / `continue` / `archive` 的既有流程；不新增显式标记文件，阶段仍由 artifacts 存在性隐式推断（零迁移成本）。

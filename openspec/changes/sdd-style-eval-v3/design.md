## Context

我们已经有：
- v1：更偏“能跑通”的 multi-style agentic eval（ison/toon/yaml），但格式差异不一定真正进入上下文
- v2：加入“格式敏感”的硬约束（强制 `Read` + 文件级编辑 + strict validate），并支持 multi-run aggregate

但 v2 的任务仍是“最小化编辑目标”，对真实 SDD 工作流的覆盖不足：真实场景通常要经历多次变更循环（`propose -> apply -> archive -> commit`），并且评估不仅关注是否通过 `validate`，还关心完成质量与偏离率。

本变更引入 v3：在 **spec-only（只写规范，不写代码）的真实小项目场景** 下做 3 次 change 迭代，并加入可选 `--judge` 评分（不作为硬门禁）。

## Goals / Non-Goals

**Goals:**
- 提供一个 v3 fixture：同一 workspace 内完成 3 次 change 循环（`propose -> apply -> archive -> commit`）
- `apply` 阶段允许使用 `llman sdd spec add-*` / `llman sdd delta add-*` 等 CLI 写入，但仍强制至少一次对 `spec.md` 的文件级编辑（避免“格式差异没进入上下文”的老问题回归）
- 继续以 `llman sdd validate --all --strict --no-interactive` 作为唯一硬门禁
- 启用 judge（claude）时输出 rubric 分数，但不影响 pass/fail；并在 batch aggregate 中统计分数分布

**Non-Goals:**
- 不实现任何真实 TODO app 代码（仅 spec-only）
- 不引入新的评测框架（继续使用 Promptfoo + Python assertions）
- 不追求跨模型/跨 provider 的通用性（先聚焦 Claude Code agent SDK 的稳定评测）

## Decisions

1) **单 testcase 覆盖 3 个 changes 循环**
   - 选择：v3 fixture 使用 1 个 test case 驱动 3 次循环，避免拆成 3 个 test case 导致 workspace 复用/上下文切割带来的偏差。

2) **Runner 预置“语义等价”的 mini-project skeleton + 3 个 change skeleton**
   - 选择：像 v2 一样在 runner 中 seed baseline，但扩展到 TODO app 场景与 3 个 changes 的初始骨架，并在各处放置确定性的 marker（例如 `TODO_V3_*`）。
   - 目的：降低 agent 自由发挥导致的方差，同时确保任务仍要求 `Read` + 文件级编辑（通过 marker 替换强制实现）。

3) **judge = score-only**
   - 选择：Promptfoo `llm-rubric` 断言设置为“永远不失败”（阈值置 0 或等价方式），仅用于输出评分字段。
   - pass/fail 仍完全由 Python assertions（包含 strict validate）决定。

4) **强制 workspace 边界（尽量不偏离）**
   - 选择：在系统提示中继续强调不得触碰父目录，同时在 Python assertions 中检查 Read/Edit/Bash 的路径痕迹（尽可能发现越界）。
   - 风险：完全阻止越界需要 provider sandbox；v3 先以“可检测 + 硬门禁”优先。

## Risks / Trade-offs

- [成本上升] 3 次循环会显著增加 tokens/turns/cost → 通过 runner 的 batch aggregate 报告与可选 `--max-turns` 控制，保持可观测与可控。
- [方差与偏离] agent 可能用 CLI 写入绕开文件级编辑 → 通过 marker 替换 + tool-call Read 检测 + spec fence 检测做硬约束。
- [Judge 不稳定] rubric 打分存在噪声 → 只做统计，不做硬门禁；多 run 聚合降低偶然性。

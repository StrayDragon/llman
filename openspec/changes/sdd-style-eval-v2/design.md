## Context

现有 `sdd-claude-style-eval` 评测（Promptfoo + `anthropic:claude-agent-sdk`）已具备：

- 3 个隔离 workspace（ison/toon/yaml）+ 临时根目录（不自动清理）
- git 快照与 `meta/` 产物
- 硬门禁：`llman sdd validate --all --strict --no-interactive`

但 testcase 路径偏“CLI 写 spec”，导致格式差异不会显著进入 agent 的上下文，也无法验证“读 spec / 改 spec”的能力与 token 成本。

## Goals / Non-Goals

**Goals:**

- 提供一个 v2 format-sensitive 评测：同一语义任务下，ison/toon/yaml 的 spec 文件内容会被 agent 读取并编辑，从而可观测不同格式对理解/修改与 token 的影响。
- 保留 v1 作为“纯 CLI 生成 + validate”基线；v2 用于“读/改 spec 文件”路径。
- 为 `--runs N` 提供聚合报告，输出每个 style 的 pass rate、turns/tokens/cost 的统计摘要（mean/median/p90）。
- 仍以 hard gate 为准（validate 失败即失败），judge rubric 仅作为可选软评分层。

**Non-Goals:**

- 不追求对 Promptfoo token 计数做二次校准（以 Promptfoo 输出为准）。
- 不尝试在单次 run 内做“严格控制变量”的学术实验（例如完全相同的 tool 输出）；我们追求工程可用与趋势可观测。
- 不在本次设计中引入新的评测框架（沿用 promptfoo + claude-agent-sdk）。

## Decisions

1) **新增 v2 fixture，而不是替换 v1**

- v1 作为稳定基线（CLI 生成 + validate），可持续回归。
- v2 强调 format-sensitive，允许更长 baseline 内容与更严格的断言（例如必须 Read spec.md）。

2) **baseline 内容由 runner 预置（而非由 agent 逐条生成）**

- runner 预置可以保证三种 style 的“语义起点”一致，减少 agent 自由发挥带来的方差。
- baseline 由 `llman sdd spec skeleton/add-*` 等命令生成，确保各 style 都是合法且由工具生成的真实结构。

3) **断言以 toolCalls + workspace 文件状态双重校验**

- 继续保留 hard gate（validate）。
- v2 新增断言：要求至少发生一次对 `llmanspec/specs/**/spec.md` 的 Read（可选再要求一次 Write/Edit），以确保格式内容进入上下文（token 计数才有意义）。

4) **聚合报告以 runner 汇总多个 work_dir 的结果为主**

- 每个 run 已产出 `promptfoo/results.json` + `meta/summary.json`。
- runner 在完成所有 runs 后生成聚合 `meta/aggregate.{json,md}`（或单独脚本），便于快速比较 mean/median/p90。

## Risks / Trade-offs

- **[更高 token/成本]** v2 强制 Read spec 文件，且 baseline 内容更长 → token 成本上升
  → 缓解：提供 `--runs` 与 `--max-turns` 控制；v2 baseline 控制在“足够长但不过度”。

- **[方差更大]** agent 可能选择不同编辑策略（CLI/手改/组合）
  → 缓解：通过 v2 断言强制 Read spec；在 prompt 中明确“以文件编辑完成关键步骤”，并通过 hard gate 保证最终正确性。

- **[断言耦合 Promptfoo 结构]** promptfoo 的 toolCalls 字段可能变化
  → 缓解：断言容忍字段变体（toolCalls/tool_calls、output/result、file_path/filePath），并保留 provider-id + env 的 fallback。

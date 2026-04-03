## Why

当前 `sdd-claude-style-eval`（Promptfoo + Claude Code agentic multi-style）评测主要依赖 `llman sdd spec add-*` / `llman sdd delta add-*` 等 CLI 写入能力，因此：

- 很难观测 **ison/toon/yaml 不同格式**对 agent “读懂/修改 spec 内容”的真实影响（agent 不必真正解析 spec 文件就能过关）。
- token 统计更像是 “工具输出 + agent话术” 的混合结果，无法把差异归因到格式本身。

我们需要一个 **format-sensitive** 的真实评测场景：同一语义任务、同一基线内容，强制 agent 读取并编辑不同格式的 `spec.md` / delta spec，从而让格式差异进入上下文与操作路径，才能可靠评估理解成本与 token 节省。

## What Changes

- 新增一个 v2 Promptfoo fixture（在 `agentdev/promptfoo/` 下）用于 format-sensitive 的 agentic eval：
  - runner 在每个 workspace 预置“语义等价但格式不同”的 baseline spec/change 内容
  - testcase 强制 agent `Read` spec 并做至少一次“文件级修改”（而非纯 CLI 追加）
  - 仍以 `llman sdd validate --all --strict --no-interactive` 作为硬门禁
- 为多次 run（`--runs N`）新增聚合报告：
  - 输出每个 style 的 turns/tokens/cost 的均值/中位数/p90（以及通过率）
  - 产物写入 work_dir 的 `meta/`（不自动清理），便于追踪与回归
- runner 支持选择 fixture 版本（v1/v2），确保现有基线可保留用于对比。

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `sdd-ab-evaluation`: 扩展评测套件，使其能够评估“格式对 agent 理解/修改能力与 token 成本”的影响（不仅是 CLI 生成能否通过 validate）。

## Impact

- `agentdev/promptfoo/**`: 新增/更新 fixture、assertion、报告聚合脚本
- `scripts/**` / `justfile`: 增加 v2 运行入口与参数（fixture 选择、聚合输出）
- 评测产物体积会增大（baseline spec 更长，meta 报告更丰富），但全部落在临时目录中且不自动清理，符合可观测性目标

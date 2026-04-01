## Why

目前仓库中已有一套 Promptfoo 评测流程（baseline vs candidate），主要用于对比“不同 SDD prompt 风格/版本在同一题目集上的回复质量”。但它本质上是 **chat-style 输出评测**，无法覆盖我们真正关心的“SDD 在真实 coding agent（Claude Code）+ 多轮交互 + 执行命令/改文件”下的行为差异与稳定性。

同时，我们最近引入了 `spec_style: ison/toon/yaml` 的多风格 SDD spec/delta。为了评估多风格策略是否能被稳定执行、是否会引入隐性失败路径、以及其 token/cost/turns 的差异，我们需要一个 **可复现、可自动化、可观测** 的评测脚手架。

因此需要新增一套基于 Promptfoo 的 Claude Code agentic 评测场景：在隔离的临时 git workspace 内运行多轮 SDD authoring/validate 流程，并产出可对比的报告与快照；同时提供 docker runner 以便在不同机器/网络环境中稳定复现。

## What Changes

- 在 `agentdev/promptfoo/` 下新增一个长流程评测 fixture（例如 `sdd_llmanspec_styles_v1/`）：
  - 使用 Promptfoo provider：`anthropic:claude-agent-sdk`（Claude Code agent）
  - 配置多轮交互（更大的 `max_turns`），并以 `permission_mode: bypassPermissions` + `allow_dangerously_skip_permissions` 允许真实执行命令/写文件
  - 对同一语义任务，分别在 `spec_style: ison/toon/yaml` 的三个隔离 workspace 中执行
  - 使用 Python assertions 做硬门禁（例如 `llman sdd validate --all --strict --no-interactive`）
  - 可选：支持 human / codex / claude 作为 judge 的软评分
- 新增 runner 脚本 `scripts/sdd-claude-style-eval.sh`：
  - runner 脚本主体位于 `agentdev/promptfoo/`（`scripts/` 可保留薄封装入口）
  - 每次运行自动创建新的临时根目录（带种子/时间戳，不自动清理）
  - 每个 style workspace 初始化为独立 git repo，生成快照（git log/diff/status）用于观测
  - 输出 `results.json/results.html` 以及 run meta（快照、日志、token/cost 汇总）
- 新增 docker 环境（Dockerfile + runner）：
  - 支持 build args 切换到阿里云镜像（apt/npm/pypi）
  - 通过挂载目录持久化输出（workspaces + results）

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `sdd-ab-evaluation`: 扩展 Promptfoo 评测套件，覆盖 Claude Code agentic 多轮执行，并加入 multi-style（ison/toon/yaml）对比与可观测快照产物。

## Impact

- 依赖与环境变量：
  - 需要 `ANTHROPIC_API_KEY`（或使用 Bedrock/Vertex 的等价环境变量）
  - 可选 judge：`OPENAI_API_KEY`（当启用 Codex rubric 时）
- 安全：
  - 评测 fixture 将显式开启“危险模式”以允许写文件/执行命令；必须限定在临时 workspace 内运行，并默认输出到隔离目录。
- 维护成本：
  - 新增 fixture 与 docker runner 需要持续维护，但可以显著降低手工评测成本并提供稳定基线。

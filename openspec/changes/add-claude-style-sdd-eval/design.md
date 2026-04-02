## Context

我们希望评测的是 **llman SDD 的真实能力**：在 coding agent（Claude Code）执行多轮交互时，能否稳定创建/修改 `llmanspec/**` 工件并通过 `llman sdd validate`；并且在 `spec_style: ison/toon/yaml` 三种承载风格下表现一致、可观测、可复现。

仓库现有 Promptfoo 评测套件更偏向“对话输出打分”（baseline vs candidate），不覆盖真实工具执行链路。与此同时，仓库已经存在 ACP pipeline（`llman x sdd-eval`），但本变更需要的是一种 **promptfoo 维度的、可复用断言与评分生态**（尤其是 human/llm rubric），用来专门评测 prompt+agent 的整体表现。

本变更将使用 Promptfoo 的 `anthropic:claude-agent-sdk` provider 直接驱动 Claude Code agent，结合 Python assertions 做硬门禁，并用 git 快照与 meta artifacts 做可观测输出。

## Goals / Non-Goals

**Goals:**
- 提供一个可自动化运行的 Claude Code agentic 评测脚手架：
  - 临时根目录（每次新 seed，不自动清理）
  - 三个 style workspace（ison/toon/yaml），互相隔离
  - 多轮交互（可配置 turns）
  - 允许真实写文件与执行命令（dangerously skip permissions）
- 提供强约束的硬门禁（Python assertions）：
  - `llman sdd validate --all --strict --no-interactive` 必须通过
  - fence/style 以及关键产物存在性检查
- 提供可观测产物：
  - workspace 为 git repo，至少输出 `git log/diff/status` 快照
  - 输出 promptfoo 的 `results.json/results.html`
  - 汇总 token/cost/turns/permission_denials（若可得）
- 提供 docker runner，支持阿里云镜像 build args，并允许挂载输出目录持久化运行产物。
 - 将评测 fixtures/runner 的“主入口”集中在 `agentdev/`，在 `scripts/` 下仅保留薄封装以提升 discoverability。

**Non-Goals:**
- 不在此变更中替代或重写 `llman x sdd-eval`（ACP pipeline）。
- 不追求对 Claude Code “每个内部 turn” 的逐回合外部快照（无法可靠拦截）；用“阶段性 git commit + meta 文件”达到可观测目的。
- 不把评测脚手架接入 CI 强制门禁（先作为 agentdev 工具落地）。

## Decisions

1) **采用 promptfoo 直接编排 Claude Code**
- 优点：复用 promptfoo 的对比/断言/报告/人工打分能力；方便与现有 prompt 评测并存。
- 代价：需要显式开启危险权限（写文件/执行命令），必须通过隔离 workspace 控制风险。

2) **硬门禁用 Python assertions，而不是完全依赖 LLM grader**
- LLM rubric 易漂移且成本更高；而 `llman sdd validate --all --strict` 是最接近“真实正确性”的硬信号。
- LLM grader 作为可选项（Codex/Claude/human），用于补充可读性、遵循流程等软指标。

3) **workspace 采用临时目录 + git repo 快照**
- 每个 style workspace 初始化为独立 git repo：
  - baseline commit（init + 配置 + skeleton）
  - 允许 agent 在关键步骤后自行 commit（写入提示词约束），以形成阶段快照
- 在 runner 的 meta 目录输出：
  - `git.log`、`git.diff`、`git.status`
  - 关键命令输出（validate/show）

4) **危险权限策略**
- Claude Code 以 bypass permissions 模式运行（等价于 `--dangerously-skip-permissions`），并显式要求 `allow_dangerously_skip_permissions`。
- 评测默认只在临时 workspace 内运行，且 runner 明确展示 work_dir，便于审计与手工清理。

5) **Docker 化作为可选运行方式**
- Dockerfile 以 build args 支持阿里云镜像（apt/npm/pypi），减少在受限网络环境下的失败率。
- runner 提供 `--docker`（或等价）模式：构建镜像并运行评测，输出目录通过 volume 挂载到宿主机。

## Risks / Trade-offs

- [风险] 危险权限导致误删/误改非 workspace 文件 → [缓解] provider `working_dir` 指向临时目录；docker 模式默认在容器内执行；脚本明确禁止把 repo root 当 workspace。
- [风险] agent 行为不稳定导致 flake → [缓解] 硬门禁只依赖 deterministic 命令（validate）；同时允许 `--repeat` 跑多次；输出保留以便定位。
- [风险] token 成本不可控 → [缓解] 提供 `max_turns`/模型配置/预算配置；报告输出 token/cost 供回归对比。

## Migration Plan

本变更为新增评测套件与工具脚本，不影响现有 CLI 行为。目录前置依赖为 `agentdev/promptfoo/`（由前置变更迁移完成）。

# sdd-ab-evaluation Specification

## Purpose
Define reproducible evaluation suites for SDD prompts and agentic workflows, including multi-style comparisons and hard-gated validation.

## Requirements
### Requirement: Built-In Old-vs-New Evaluation Flow
The SDD workflow MUST provide an evaluation flow that compares legacy and new style outputs on the same scenario set.

#### Scenario: Run evaluation over shared scenarios
- **WHEN** a user executes the evaluation flow for a target scenario set
- **THEN** the system runs both legacy and new style generation/evaluation on equivalent inputs
- **AND** records paired results for comparison

### Requirement: 提供可复现的 SDD prompts Promptfoo 评估套件
SDD workflow MUST 提供一套可复现的 Promptfoo 评估套件，用于对比不同风格/版本的 SDD prompt（baseline vs candidate）。该评估套件（fixtures + 默认模型列表）MUST 存放在仓库顶层 `agentdev/promptfoo/`（而不是 `artifacts/`），并且评估流程 MUST 支持在隔离的临时目录下运行以避免触碰真实用户配置。

#### Scenario: 评估在隔离配置目录下运行
- **WHEN** 维护者运行评测脚本（例如 `bash scripts/sdd-prompts-eval.sh`），并从 `agentdev/promptfoo/` 读取 promptfoo fixtures 与默认模型列表
- **THEN** Promptfoo 产生的数据（例如 `.promptfoo/` 与导出的 `results.*`）均写入该评估流程创建的临时工作目录
- **AND** 不修改用户真实配置目录（仅使用显式指定的 `LLMAN_CONFIG_DIR`）

### Requirement: Safety-First Scoring Output
Evaluation outputs MUST prioritize safety and quality signals over cost metrics (for example, token/latency).

#### Scenario: Report includes prioritized metrics
- **WHEN** the evaluation report is generated
- **THEN** it includes quality and safety scores before token/latency metrics
- **AND** it marks pass/fail gates for safety-sensitive checks

### Requirement: Promptfoo fixtures 与 artifacts 目录语义分离
仓库 MUST 使用 `agentdev/` 作为 agent/prompt 相关开发与评测资产的归属目录；`artifacts/` MUST 仅用于测试配置 fixture、schema 产物或其他“可执行/可复用”的非评测资产。Promptfoo fixtures MUST NOT 以长期可执行入口的形式落在 `artifacts/**/promptfoo` 下。

#### Scenario: Promptfoo 评测套件位于 agentdev
- **WHEN** 维护者在仓库中查找 promptfoo 评测套件位置
- **THEN** 评测套件位于 `agentdev/promptfoo/`
- **AND** `artifacts/` 下不再作为 promptfoo fixtures 的稳定入口

### Requirement: Claude Code agentic 评测可通过 Promptfoo 自动运行
SDD workflow MUST 提供一个可复现的评测脚手架，用于通过 Promptfoo 驱动 Claude Code agent 在隔离 workspace 内进行多轮交互，并允许真实执行命令/写文件以模拟真实开发过程。该脚手架 MUST 使用确定性硬门禁（例如 `llman sdd validate --all --strict --no-interactive`）来判断评测是否通过。

#### Scenario: 运行一次 Claude Code agentic 评测
- **WHEN** 维护者运行一个 Promptfoo fixture 来驱动 Claude Code agent 完成一轮 SDD authoring + validate 流程
- **THEN** 评测输出包含 `results.json` 与 `results.html`
- **AND** 评测以 `llman sdd validate --all --strict --no-interactive` 作为硬门禁（失败则整体评测失败）

### Requirement: Format-Sensitive Agentic Tasks
The evaluation suite MUST include at least one agentic task that requires reading and editing the style-specific spec files (main spec and/or delta spec), so that format differences (ison/toon/yaml) materially affect the agent’s context and actions.

#### Scenario: Agent reads and edits main spec file
- **WHEN** the multi-style agentic eval runs for `spec_style: ison|toon|yaml`
- **THEN** the agent reads `llmanspec/specs/**/spec.md` in the current workspace
- **AND** makes a file-level edit that changes the spec content
- **AND** `llman sdd validate --all --strict --no-interactive` passes

#### Scenario: Agent reads and edits delta spec file
- **WHEN** the eval includes a change under `llmanspec/changes/**`
- **THEN** the agent reads `llmanspec/changes/**/specs/**/spec.md`
- **AND** makes a file-level edit that changes the delta content
- **AND** `llman sdd validate --all --strict --no-interactive` passes

### Requirement: Multi-style（ison/toon/yaml）在同一任务集下可对比
评测脚手架 MUST 在同一语义任务集下分别运行 `spec_style: ison`、`spec_style: toon`、`spec_style: yaml` 的三个隔离 workspace，并输出每种风格的通过情况与关键指标（例如 turns/token/cost）。

#### Scenario: 同一任务在三种 spec_style 下跑通
- **WHEN** 维护者执行 multi-style 评测
- **THEN** 系统为 ison/toon/yaml 创建三个隔离 workspace 并分别运行相同任务
- **AND** 输出报告包含每个 style 的 pass/fail 与基础指标

### Requirement: Seeded Baseline Content Is Semantically Equivalent Across Styles
The eval runner MUST pre-seed each style workspace with semantically equivalent baseline specs/changes before starting Promptfoo evaluation, so that outcomes can be compared with reduced variance.

#### Scenario: Runner seeds baseline before evaluation
- **WHEN** a new eval run starts
- **THEN** the runner creates three isolated workspaces (`ison/toon/yaml`)
- **AND** seeds the same logical capability + change content in each workspace
- **AND** the seeded content is style-correct for that workspace

### Requirement: 每次评测 run 产出可观测快照与元数据
评测脚手架 MUST 为每次运行创建独立的临时根目录（不自动清理），并在其中产出可观测快照（推荐使用 git repo 形式）与元数据目录（meta）。元数据 MUST 至少包含 workspace 的 `git log/diff/status`（或等价快照）以及关键命令输出（validate/show）。

#### Scenario: 评测产物包含 meta 快照
- **WHEN** 一次评测运行结束
- **THEN** 临时根目录下存在 `meta/`（或等价）用于保存快照与日志
- **AND** 其中包含每个 workspace 的 `git log/diff/status`（或等价信息）

### Requirement: Multi-Run Aggregate Metrics Report
When the runner is executed with `--runs N` (N ≥ 2), it MUST generate an aggregate report that summarizes pass rate and token/turn/cost distributions per style across runs.

#### Scenario: Aggregation outputs a batch report
- **WHEN** a maintainer runs `just sdd-claude-style-eval --runs 10` (or equivalent)
- **THEN** the runner writes an aggregate summary to a batch-level `meta/aggregate.md` (and/or `meta/aggregate.json`)
- **AND** the report includes at least: pass rate, mean/median/p90 of total tokens and turns per style

### Requirement: 支持 human / Codex / Claude 的可选软评分
评测脚手架 SHOULD 支持可选的软评分层：人工打分（human）与 LLM rubric（例如 Codex/Claude judge）。该软评分层 MUST 可选，且不得替代硬门禁（validate）作为唯一通过条件。

#### Scenario: 同时启用硬门禁与可选 rubric
- **WHEN** 维护者启用 rubric judge（Codex 或 Claude）或 human judge
- **THEN** 评测仍以硬门禁为基础，同时输出 rubric/human 的评分结果

### Requirement: 提供可复现的 Docker runner（支持阿里云镜像参数）
评测脚手架 MUST 提供一个 docker 运行方式，用于在受限网络/不同机器上复现。Dockerfile MUST 支持通过 build args 配置镜像源（至少覆盖 apt、npm、pypi）以切换到阿里云 mirror。docker runner MUST 支持挂载输出目录以持久化 `results.*` 与 workspaces。

#### Scenario: 使用 docker + aliyun mirror 构建并运行评测
- **WHEN** 维护者使用传入的 build args（apt/npm/pypi mirror）构建 docker 镜像并运行评测
- **THEN** 评测可在容器内完成
- **AND** 输出目录通过 volume 挂载持久化到宿主机

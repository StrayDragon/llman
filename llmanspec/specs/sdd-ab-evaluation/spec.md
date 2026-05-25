---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - llman sdd validate sdd-ab-evaluation --type spec --strict --no-interactive
llman_spec_evidence:
  - migrated from openspec
---

```toon
kind: llman.sdd.spec
name: "sdd-ab-evaluation"
purpose: "Define reproducible evaluation suites for SDD prompts and agentic workflows, including multi-style comparisons and hard-gated validation."
requirements[12]{req_id,title,statement}:
  r1,"Built-In Old-vs-New Evaluation Flow",The SDD workflow MUST provide an evaluation flow that compares legacy and new style outputs on the same scenario set.
  r2,提供可复现的 SDD prompts Promptfoo 评估套件,SDD workflow MUST 提供一套可复现的 Promptfoo 评估套件，用于对比不同风格/版本的 SDD prompt（baseline vs candidate）。该评估套件（fixtures + 默认模型列表）MUST 存放在仓库顶层 `agentdev/promptfoo/`（而不是 `artifacts/`），并且评估流程 MUST 支持在隔离的临时目录下运行以避免触碰真实用户配置。
  r3,"Safety-First Scoring Output","Evaluation outputs MUST prioritize safety and quality signals over cost metrics (for example, token/latency)."
  r4,Promptfoo fixtures 与 artifacts 目录语义分离,仓库 MUST 使用 `agentdev/` 作为 agent/prompt 相关开发与评测资产的归属目录；`artifacts/` MUST 仅用于测试配置 fixture、schema 产物或其他“可执行/可复用”的非评测资产。Promptfoo fixtures MUST NOT 以长期可执行入口的形式落在 `artifacts/**/promptfoo` 下。
  r5,Claude Code agentic 评测可通过 Promptfoo 自动运行,"SDD workflow MUST 提供一个可复现的评测脚手架，用于通过 Promptfoo 驱动 Claude Code agent 在隔离 workspace 内进行多轮交互，并允许真实执行命令/写文件以模拟真实开发过程。该脚手架 MUST 使用确定性硬门禁（例如 `llman sdd validate --all --strict --no-interactive`）来判断评测是否通过。"
  r6,"Format-Sensitive Agentic Tasks","The evaluation suite MUST include at least one agentic task that requires reading and editing the style-specific spec files (main spec and/or delta spec), so that format differences (ison/toon/yaml) materially affect the agent’s context and actions."
  r7,"Multi-style（ison/toon/yaml）在同一任务集下可对比","评测脚手架 MUST 在同一语义任务集下分别运行 `spec_style: ison`、`spec_style: toon`、`spec_style: yaml` 的三个隔离 workspace，并输出每种风格的通过情况与关键指标（例如 turns/token/cost）。"
  r8,Seeded Baseline Content Is Semantically Equivalent Across Styles,"The eval runner MUST pre-seed each style workspace with semantically equivalent baseline specs/changes before starting Promptfoo evaluation, so that outcomes can be compared with reduced variance."
  r9,每次评测 run 产出可观测快照与元数据,评测脚手架 MUST 为每次运行创建独立的临时根目录（不自动清理），并在其中产出可观测快照（推荐使用 git repo 形式）与元数据目录（meta）。元数据 MUST 至少包含 workspace 的 `git log/diff/status`（或等价快照）以及关键命令输出（validate/show）。
  r10,"Multi-Run Aggregate Metrics Report","When the runner is executed with `--runs N` (N ≥ 2), it MUST generate an aggregate report that summarizes pass rate and token/turn/cost distributions per style across runs."
  r11,支持 human / Codex / Claude 的可选软评分,评测脚手架 SHOULD 支持可选的软评分层：人工打分（human）与 LLM rubric（例如 Codex/Claude judge）。该软评分层 MUST 可选，且不得替代硬门禁（validate）作为唯一通过条件。
  r12,提供可复现的 Docker runner（支持阿里云镜像参数）,评测脚手架 MUST 提供一个 docker 运行方式，用于在受限网络/不同机器上复现。Dockerfile MUST 支持通过 build args 配置镜像源（至少覆盖 apt、npm、pypi）以切换到阿里云 mirror。docker runner MUST 支持挂载输出目录以持久化 `results.*` 与 workspaces。
scenarios[13]{req_id,id,given,when,then}:
  r1,"run-evaluation-over-shared-scenarios","",a user executes the evaluation flow for a target scenario set,the system runs both legacy and new style generation/evaluation on equivalent inputs
  r2,评估在隔离配置目录下运行,"","维护者运行评测脚本（例如 `bash scripts/sdd-prompts-eval.sh`），并从 `agentdev/promptfoo/` 读取 promptfoo fixtures 与默认模型列表",Promptfoo 产生的数据（例如 `.promptfoo/` 与导出的 `results.*`）均写入该评估流程创建的临时工作目录
  r3,"report-includes-prioritized-metrics","",the evaluation report is generated,it includes quality and safety scores before token/latency metrics
  r4,"promptfoo-评测套件位于-agentdev","",维护者在仓库中查找 promptfoo 评测套件位置,评测套件位于 `agentdev/promptfoo/`
  r5,"运行一次-claude-code-agentic-评测","",维护者运行一个 Promptfoo fixture 来驱动 Claude Code agent 完成一轮 SDD authoring + validate 流程,评测输出包含 `results.json` 与 `results.html`
  r6,"agent-reads-and-edits-main-spec-file","","the multi-style agentic eval runs for `spec_style: ison|toon|yaml`",the agent reads `llmanspec/specs/**/spec.md` in the current workspace
  r6,"agent-reads-and-edits-delta-spec-file","",the eval includes a change under `llmanspec/changes/**`,the agent reads `llmanspec/changes/**/specs/**/spec.md`
  r7,"同一任务在三种-spec-style-下跑通","","维护者执行 multi-style 评测",系统为 ison/toon/yaml 创建三个隔离 workspace 并分别运行相同任务
  r8,"runner-seeds-baseline-before-evaluation","",a new eval run starts,the runner creates three isolated workspaces (`ison/toon/yaml`)
  r9,"评测产物包含-meta-快照","",一次评测运行结束,临时根目录下存在 `meta/`（或等价）用于保存快照与日志
  r10,"aggregation-outputs-a-batch-report","","a maintainer runs `just sdd-claude-style-eval --runs 10` (or equivalent)","the runner writes an aggregate summary to a batch-level `meta/aggregate.md` (and/or `meta/aggregate.json`)"
  r11,"同时启用硬门禁与可选-rubric","",维护者启用 rubric judge（Codex 或 Claude）或 human judge,评测仍以硬门禁为基础，同时输出 rubric/human 的评分结果
  r12,"使用-docker-aliyun-mirror-构建并运行评测","",维护者使用传入的 build args（apt/npm/pypi mirror）构建 docker 镜像并运行评测,评测可在容器内完成
```

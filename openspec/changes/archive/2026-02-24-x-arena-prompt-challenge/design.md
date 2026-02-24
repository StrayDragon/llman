## Context

当前 `llman` 已能：
- 管理并注入 `codex`/`claude-code` prompts（见 `llman prompts`）。
- 管理 `llman x codex` 与 `llman x claude-code` 的多账号配置（以写入/注入 env 的方式执行外部 CLI）。

但当我们希望“调优 prompt”时，缺少一个可重复的评估框架来回答：
- prompt A 与 prompt B 在同一任务集上哪个更好？
- 不同模型下同一 prompt 的差异有多大？
- 对 `coding agent` 场景（改仓库）如何引入客观信号（能否应用 patch、测试是否通过）？

约束：
- 测试与开发命令不得触碰真实用户配置；必须可通过 `LLMAN_CONFIG_DIR` 隔离。
- 不引入大框架/重写；尽量增量、可 review。
- 跨平台优先 Linux/macOS（Windows 非重点）。

## Goals / Non-Goals

**Goals:**
- 新增实验性命令 `llman x arena`，提供 prompt/模型大乱斗的最小可用闭环：
  - `/models` 发现：读取 `OPENAI_*` 环境变量并请求 `GET /v1/models`，让用户多选参赛模型。
  - 比赛配置：contest/dataset 配置落盘在 `LLMAN_CONFIG_DIR/arena/`。
  - 批量生成：`gen` 将多 prompt（×多模型）在任务集上跑出 A/B 对战样本并落盘。
  - 回放投票：`vote` 逐场回放，记录人类 winner（A/B/tie/skip）。
  - 报告排名：`report` 基于投票计算 Elo 并输出 leaderboard 与可复查的 run 产物。
- 同时支持两类任务：
  - `text`：对输出直接投票。
  - `repo`：模型输出统一 diff → 在临时目录复制 dataset 指定 repo 模板目录 → 自动 apply → 运行验证命令 → 将客观结果展示给投票者。
- 网络/LLM 能力默认编译可用（不通过 feature gate 控制），以降低使用摩擦。

**Non-Goals:**
- 不在 MVP 中实现“自动 prompt 进化/RL 优化器”（仅沉淀数据与排名）。
- 不在 MVP 中直接驱动 `codex`/`claude` CLI 作为工具执行型 runner（避免把评估变成“工具交互差异评估”）。
- 不追求全 Windows 体验一致性（仅保证不崩溃且错误明确）。

## Decisions

### Decision: 命令入口放在 `llman x arena`（实验性）
**Why:** 现阶段是探索/迭代特性，放在 `x` 下避免污染稳定 CLI；未来成熟后可考虑提升到顶层。

**Alternatives considered:**
- `llman prompts challenge`：更贴近 prompts 管理，但会把评估/落盘/runner 等复杂度塞进 prompts 子系统。

### Decision: 模型发现使用 `OPENAI_*` env + `GET /v1/models`
**Why:** 用户明确希望读取 `OPENAI_` 环境变量，并可直接支持 OpenAI-compatible 网关；且能让 contest 以“模型列表多选”方式配置参赛者。

**Alternatives considered:**
- 从 `llman x codex` 的 provider groups 拉取：适用于多网关/多账号，但会把“Codex 配置管理”与“arena 生成”耦合得更紧，MVP 暂不做。

### Decision: Arena 网络/LLM 能力默认编译（不使用 feature gate）
**Why:** 当前主要由作者自用，优先降低构建与运行摩擦；后续若需要减重或面向更广泛用户，再考虑引入可选 feature。

**Alternatives considered:**
- 通过 `arena-ai` feature gate 按需启用网络/LLM 依赖与相关子命令。

### Decision: 参赛者定义为 `prompt × model` 的笛卡尔积
**Why:** 允许同时比较 prompt 与 model；数据结构简单且易扩展。

### Decision: 评估闭环使用人类投票 + Elo
**Why:** MVP 追求稳定可靠、可解释。LLM-as-judge 虽可自动化，但容易引入偏差与漂移，且会让“调 prompt”变成“调 judge prompt”。

**Alternatives considered:**
- LLM judge + rubric：后续可作为可选模式引入（并与人类投票数据对齐验证）。

### Decision: `repo` 任务使用 “API 生成 patch → arena apply → 跑验证命令”
**Why:** 客观、可重复；并能在投票时展示 pass/fail 结果，减少主观噪声。

**Alternatives considered:**
- 让 `codex`/`claude-code` CLI 在 tmp workspace 自主行动：更贴近真实 agent，但需要稳定的非交互运行协议与输出契约，且难以保证可重复性。

### Decision: run 产物落盘使用文本友好的 JSONL/Markdown
**Why:** 便于追加、合并、diff 与离线回放；避免一开始引入 DB 迁移/锁等复杂度。

## Risks / Trade-offs

- **[成本与速率]** 大规模对战会消耗大量 token 与时间 → 提供 `--rounds`、`--seed`、以及可选的去重/复用策略（仅在 `temperature=0` 时启用）。
- **[可重复性]** 生成存在随机性 → 默认 `temperature=0`，并记录生成参数与 seed 到 run 元信息。
- **[安全性]** `repo` 任务需要执行验证命令，存在破坏性命令风险 → MVP 仅允许用户显式在 contest/task 中配置命令；并在实现阶段加入最小的危险模式检测与明确提示（复用现有安全检查思路）。
- **[兼容性]** `/models` 在不同网关实现不一致 → 在实现阶段对非 200/非预期 JSON 做明确错误输出，并允许用户直接在 contest 中手写 `models=[...]` 作为 fallback。

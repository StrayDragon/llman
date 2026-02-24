## Why

目前 `llman` 已经能管理并注入 `codex`/`claude-code` 的 prompts，但我们缺少一种可重复、可积累的数据闭环来评估“某个 prompt 变体对任务结果的真实影响”，导致 prompt 调优主要依赖直觉与零散试跑。

我们希望新增一个“模型大乱斗 / prompts challenge”能力：对同一组任务，将多个 prompts（以及可选的多个模型）进行批量对战与回放投票，并沉淀可查询的排名与对战记录，从而让 prompt 调优变成可量化工程。

## What Changes

- 新增实验性子命令 `llman x arena`，用于 prompt/模型挑战赛的全流程管理：
  - 从 `OPENAI_*` 环境变量读取配置并请求 `GET /v1/models`，供用户列出/多选参赛模型。
  - 读取 `llman prompts` 管理的 prompts（`codex`/`claude-code`）作为参赛 prompt 变体来源。
  - 支持创建 contest/dataset 配置、批量生成 run 结果、回放投票、生成 Elo 排名与报告。
- 支持两类任务：
  - `text`：直接对输出进行 A/B 对比投票。
  - `repo`：模型输出 patch（统一 diff）→ 在临时目录复制 dataset 指定的 repo 模板目录 → 自动应用 patch → 执行验证命令（默认由 contest 指定、task 可覆盖）→ 将客观验证结果与输出一起用于投票与报告。
- 引入（或复用现有）OpenAI-compatible API 客户端能力用于生成与 `/models` 发现（计划优先复用 `adk-rust`；如 `adk-rust` 未覆盖 `/models`，则补充最小 HTTP 客户端实现）。

## Capabilities

### New Capabilities
- `arena-prompt-challenge`: 提供 `llman x arena` 的 contest/dataset/run/vote/report 工作流，用于对 prompts（及可选多模型）进行批量对战、投票与 Elo 排名，并支持 `text` 与 `repo` 两类任务（repo 任务包含 patch 应用与客观验证）。

### Modified Capabilities

<!-- none -->

## Impact

- CLI
  - 新增 `llman x arena` 子命令（实验性），并增加相应帮助/错误输出与交互体验。
- 配置与落盘
  - 在 `LLMAN_CONFIG_DIR` 下新增 `arena/` 数据目录（contests/datasets/runs 等），遵循现有“测试/开发不触碰真实用户配置”的约束。
- 依赖与构建
  - 默认构建将包含用于 `/models` 与生成的网络/LLM 依赖（优先复用 `adk-rust`；必要时补充最小 HTTP 客户端依赖），不再通过可选 feature 控制。
- 测试
  - 增加对 Elo 计算、配置解析、run 落盘与 repo 任务客观验证（temp workspace）等的单元/集成测试，确保不会污染仓库与用户环境。

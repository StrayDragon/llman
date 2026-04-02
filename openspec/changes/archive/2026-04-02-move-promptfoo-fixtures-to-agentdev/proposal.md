## Why

目前仓库里的 Promptfoo fixtures 放在 `artifacts/testing_config_home/promptfoo/` 下，但 `artifacts/` 在本项目里更偏向“可执行的测试配置 fixture / schema 产物”，并不适合作为“agent dev / prompt 实验”的长期落点。随着我们要引入更复杂的 SDD 评测（Claude Code、多轮交互、multi-style 等），继续把评测套件放在 `artifacts/` 会让目录语义越来越混乱，也会增加后续 docker/runner 的组织成本。

因此需要先做一次“目录语义重整”：把 promptfoo 相关评测套件迁移到一个新的顶层 `agentdev/` 目录下，作为开发与评测的稳定入口。

## What Changes

- 新增仓库顶层 `agentdev/` 目录，用于承载 agent/prompt 相关的开发与评测资产（fixtures、脚本、docker 等）。
- 将现有 Promptfoo fixtures 从 `artifacts/testing_config_home/promptfoo/` 迁移到 `agentdev/promptfoo/`：
  - `default_models.txt`
  - `sdd_apply_v1/`（以及其 prompts/tests/config）
- 评测相关脚本与文档优先放入 `agentdev/`（必要时保留 `scripts/` 下的薄封装入口），并更新评测脚本以引用新的路径，不再依赖 `artifacts/**/promptfoo`。
- 更新 OpenSpec 规范（`sdd-ab-evaluation`）以反映新的目录契约与隔离运行方式。

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `sdd-ab-evaluation`: Promptfoo 评估套件的位置从 `artifacts/` 迁移到 `agentdev/`，并明确目录与隔离运行约束。

## Impact

- 仓库目录结构：
  - 新增 `agentdev/`；
  - 删除/迁移 `artifacts/testing_config_home/promptfoo/`（仅保留 `artifacts/testing_config_home` 用作测试配置 fixture）。
- 脚本与文档：
  - 更新 `scripts/sdd-prompts-eval.sh` 的 fixtures 与默认模型列表路径；
  - 后续新增的评测套件（例如 Claude Code agentic eval）统一落在 `agentdev/` 下。
- 兼容性：
  - 这是一次 repo 内部路径迁移；旧路径不再作为兼容入口（避免双写/双维护）。

# SDD 提示词工程（模板优化）工作流

本文面向 **Prompt/Context 工程师**：你只需要修改模板文本（`templates/**`），然后用可复现的流程跑门禁与 Arena 评估，不需要理解 Rust 细节。

## 两类贡献者如何解耦

### Prompt/Context 工程师（文本为主）
- 主要改动范围：
  - new：`templates/sdd/**`
  - legacy：`templates/sdd-legacy/**`（与 new 同步维护）
  - 共享注入单元：`templates/**/units/**`
- 每轮改动推荐顺序：
  1) `just check-sdd-templates`
  2) `cargo +nightly test --test sdd_integration_tests -q`（如本轮涉及模板结构/渲染契约）
  3) `llman sdd validate --ab-report --json --no-interactive`（快速门禁：风格对比/治理字段）
  4) `just sdd-prompts-eval`（深评估：Arena）

### 项目开发者（工具/门禁为主）
- 负责在需要时修改：
  - `src/sdd/**`（渲染出口、生成链路、门禁）
  - `tests/**`（锁定关键不变式）
  - `scripts/**`/`justfile`（降低 prompt 工程师迭代摩擦）

## 环境变量（Arena 必需）

至少需要：
- `OPENAI_API_KEY`

推荐统一指定评估模型（避免不同运行不可比）：
- `OPENAI_DEFAULT_MODEL=gpt-5.2`
  - 评估入口脚本会优先使用它；若未设置，可通过脚本参数 `--model` 显式指定。

如需中转/加速：
- `OPENAI_BASE_URL=https://<your-proxy>/v1`（推荐；需带 `/v1`）
  - `OPENAI_API_BASE` 也可用（`llman x arena` 支持）

## 一键深评估（推荐）

运行：
```bash
just sdd-prompts-eval --rounds 10
```

该入口会：
1) 在 `.tmp/` 下创建临时目录（不污染仓库根目录）
2) 生成 baseline/candidate 的 system prompt（默认：legacy vs new）
3) 复制 Arena fixtures 到临时 `LLMAN_CONFIG_DIR`
4) 运行 `llman x arena gen`

脚本会输出本次使用的 `LLMAN_CONFIG_DIR`。随后你可以继续：
```bash
LLMAN_CONFIG_DIR=<上一步输出的路径> cargo run -q -- x arena vote --run <RUN_ID>
LLMAN_CONFIG_DIR=<上一步输出的路径> cargo run -q -- x arena report --run <RUN_ID>
```

## 关于 `templates/sdd/**/skills/shared.md`

`templates/sdd/*/skills/shared.md`（以及 legacy 对应文件）不参与 `llman sdd update-skills` 生成链路（不在 `SKILL_FILES` 中），继续保留容易造成“影子真源”的误解；推荐在本变更内清理（删除或改为指针页，真源统一到 `templates/**/units/**`）。

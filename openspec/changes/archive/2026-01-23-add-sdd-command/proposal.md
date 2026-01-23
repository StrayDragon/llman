# 变更：新增 sdd 子命令

## Why
llman 目前缺少原生的规范驱动开发（Spec-Driven Development, SDD）流程。我们希望在仓库内实现一个无外部依赖的方式来创建、校验、归档 OpenSpec 风格的变更规范，让 SDD 能在 llman 中稳定、可复用地执行。

## What Changes
- 增加顶层 `llman sdd` 命令组，覆盖 OpenSpec 核心流程（init / update / list / show / validate / archive）。
- 内置 spec-driven 模板（proposal/spec/design/tasks），写入 `llmanspec/templates/spec-driven/`，保持与 OpenSpec schema 对齐。
- 实现变更/规范发现、校验与归档，使 `llmanspec/specs` 作为唯一真相来源，并与 `openspec/` 共存互不影响。
- `llman sdd update` 写入并维护 `llmanspec/AGENTS.md` 中的 LLMANSPEC 受管块，提醒 agent 在特定场景使用 `llman sdd` 管理 specs。
- `list / show / validate` 支持 `--json` 输出并与 OpenSpec CLI 结构对齐（含 `list --specs --json`）。
- `archive` 支持 `--dry-run` 预检查合并与移动的影响。
- 增加测试与文档，覆盖新命令与关键行为。

## 能力
### 新增能力
- `sdd-workflow`：规范驱动开发工作流与 CLI 支持。

### 修改能力
- （无）

## 影响
- 影响的规范：新增 `sdd-workflow`；现有 CLI 行为仍需满足 `cli-experience` 与 `errors-exit`。
- 影响的代码：`src/cli.rs`、新增 `src/sdd/**`、`locales/app.yml`、`README.md`、`tests/**`、模板资源（新增 `llmanspec/` 输出）。

## 参考
- https://github.com/StrayDragon/OpenSpec/blob/main/README.md
- https://github.com/StrayDragon/OpenSpec/blob/main/schemas/spec-driven/schema.yaml
- https://github.com/StrayDragon/OpenSpec/blob/main/schemas/spec-driven/templates/proposal.md
- https://github.com/StrayDragon/OpenSpec/blob/main/schemas/spec-driven/templates/spec.md
- https://github.com/StrayDragon/OpenSpec/blob/main/docs/experimental-workflow.md
- https://github.com/StrayDragon/OpenSpec/blob/main/src/cli/index.ts

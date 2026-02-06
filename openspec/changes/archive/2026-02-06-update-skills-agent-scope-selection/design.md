## Context
当前 `llman skills` 交互第一步直接展示 `target id`（如 `claude_user`）。这个设计对“按工具 + 范围管理技能”的用户目标不友好，也无法体现 Claude 与 Codex 在官方文档里的 scope 语义。

用户反馈的核心诉求有三点：
1. 先选工具（claude/codex/_agentskills_），再选范围（personal/project 或 user/repo）。
2. 项目内技能与个人技能可分开管理，降低上下文负担。
3. 范围命名要贴近各工具文档语义，避免暴露内部 target id。

## Goals / Non-Goals
- Goals
  - 将交互入口改为 `agent -> scope -> skills`。
  - 为 Claude/Codex 提供语义化 scope 文案。
  - 在默认配置下提供可用的项目范围 target（仓库内）。
  - 保持 `config.toml` v2 与 `registry.json` 兼容。
- Non-Goals
  - 不实现 Admin/System 级管理。
  - 不改变 `<skills_root>` 单一扫描源。
  - 不引入非交互参数扩展。

## Decisions

### 1) 交互流程改为 Agent First
- 第一步：`Select which agent tools to manage`。
- 第二步：`Select scope to manage`（按 agent 定制文案）。
- 第三步：`Select skills`（默认勾选仍基于目标目录真实链接状态）。

内部仍落到单个 `target` 执行差异同步，因此 registry 与链接逻辑保持最小改动。

### 2) Scope 文案与内部字段解耦
继续沿用配置中的 `agent`、`scope`、`id` 作为内部标识；交互层新增“显示标签映射”：
- `claude/user` -> `Personal (All your projects)`
- `claude/project` -> `Project (This project only)`
- `codex/user` -> `User (All your projects)`
- `codex/repo` -> `Repo (This project only)`
- `agent/global` -> `Global`

未命中映射的组合，回退为 `<scope> (<target-id>)`，保证兼容自定义 target。

### 3) 默认 target 扩展为“用户 + 项目”双层
在无 `config.toml` 时，默认 target 扩展如下：
- Claude：`user` + `project`
- Codex：`user` + `repo`
- AgentSkills：`global`

项目范围路径基于 git 根目录：
- Claude project: `<repo_root>/.claude/skills`
- Codex repo: `<repo_root>/.agents/skills`

若不在 git 仓库内，项目范围 target 以 `mode=skip` 提供（只读展示，不可写入）。

### 4) Codex 用户路径向 `.agents/skills` 对齐
Codex USER 目标路径优先使用 `~/.agents/skills`（或 `CODEX_HOME/.agents/skills`），并兼容已有 `~/.codex/skills`（或 `CODEX_HOME/skills`）回退。

该决策用于对齐 Codex skills 官方文档（USER/REPO 均在 `.agents/skills` 体系下），同时避免破坏已有用户目录。

## Alternatives considered
- 继续展示 target id：拒绝，用户认知成本高。
- 完全统一命名（全部使用 User/Project）：拒绝，会偏离 Claude 当前术语习惯。
- 直接迁移所有 Codex 用户目录到 `.agents/skills`：拒绝，迁移风险高。

## Risks / Trade-offs
- 默认 target 数量增加后，交互逻辑更复杂，需要补足单元测试。
- Codex 用户路径双策略（新优先 + 旧回退）会增加路径判定分支。
- 非仓库场景下项目 scope 只读显示，需确保提示清晰，避免误解。

## Verification strategy
- 单元测试：
  - 默认 target 生成（仓库内/仓库外）
  - Codex USER 路径优先与回退
  - agent/scope 显示标签映射
- 集成测试：
  - 非交互模式回归（链接与 registry 行为不变）
  - 交互流程关键路径（菜单顺序与取消 no-op）

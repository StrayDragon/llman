# LLMAN 规范驱动开发 (SDD)

这些说明适用于在此仓库中工作的 AI 助手。

当请求满足以下条件时：
- 提及 proposal/spec/change/plan
- 引入新功能、破坏性变更、架构转变或大型性能/安全工作
- 模棱两可且需要权威规范

请使用 llmanspec 与 `llman sdd` 工作流。

常用命令：
- `llman sdd list`
- `llman sdd show <item>`
- `llman sdd validate <id> --strict --no-interactive`
- `llman sdd archive <id>`
- `llman sdd update-skills --all`

项目上下文：
- `llmanspec/project.md` 记录约定与约束。

Locale + skills：
- `llmanspec/config.yaml` 设置 `locale` 与 skills 路径。
- locale 仅影响模板与 skills，CLI 输出保持英文。
- 使用 `llman sdd update-skills` 刷新技能。

仅使用 AGENTS.md 的上下文注入方式，不要添加工具专用的 slash commands。

## 阶段 1：创建变更
在以下情况创建提案：
- 新能力或功能
- 破坏性变更（API、schema）
- 架构或模式调整
- 会改变行为的性能/安全工作

以下情况可跳过提案：
- 修复 bug（恢复预期行为）
- 拼写/格式/注释
- 非破坏性依赖更新
- 仅配置变更

工作流程：
1. 阅读 `llmanspec/project.md`。
2. 查看现有状态：`llman sdd list` 与 `llman sdd list --specs`。
3. 选择唯一的 change id：kebab-case + 动词前缀（`add-`、`update-`、`remove-`、`refactor-`）。
4. 创建 `llmanspec/changes/<change-id>/`，包含 `proposal.md`、`tasks.md` 和可选的 `design.md`。
5. 为每个受影响能力添加 `llmanspec/changes/<change-id>/specs/<capability>/spec.md`，使用：
   - `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`
   - 每个 requirement 至少包含一个 `#### Scenario:`
6. 校验：`llman sdd validate <change-id> --strict --no-interactive`。

## 阶段 2：实施变更
将以下步骤作为 TODO 并按顺序完成。
1. 阅读 `proposal.md`。
2. 如存在则阅读 `design.md`。
3. 阅读 `tasks.md`。
4. 按顺序实施任务。
5. 仅在完成后勾选 `tasks.md`。
6. 提案批准前不要开始实施。

## 阶段 3：归档变更
部署后：
- 运行 `llman sdd archive <change-id>`。
- 仅工具类变更使用 `--skip-specs`。
- 再次校验：`llman sdd validate --strict --no-interactive`。

## 规范格式要点
- spec 必须包含 YAML frontmatter：
  - `llman_spec_valid_scope`
  - `llman_spec_valid_commands`
  - `llman_spec_evidence`
- 每条 requirement 文本必须包含 `SHALL` 或 `MUST`。
- 场景标题必须使用 `#### Scenario:`。

保留此托管块，便于 `llman sdd update` 刷新。

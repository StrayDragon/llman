<!-- llman-template-version: 1 -->
<!-- region: sdd-commands -->
常用命令：
- `llman sdd list`（列出变更）
- `llman sdd list --specs`（列出 specs）
- `llman sdd show <id>`（查看 change/spec）
- `llman sdd validate <id>`（校验变更或 spec）
- `llman sdd validate --all`（批量校验）
- `llman sdd archive run <id>`（归档变更）
- `llman sdd archive <id>`（`archive run` 的兼容别名）
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]`（将已归档目录冻结到单一冷备文件）
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]`（从冷备文件恢复目录）
<!-- endregion -->

<!-- region: llman-sdd-quickstart -->
llman sdd 工作流：
- 安装/刷新：`llman sdd update-skills --all`
- Claude Code 命令绑定位置：`.claude/commands/llman-sdd/`
- Codex 不生成 llman sdd slash-command/custom-prompt 绑定；请使用 `llman-sdd-*` skills。

常见动作：
- `/llman-sdd:new <id|description>` → 创建 `llmanspec/changes/<id>/`
- `/llman-sdd:continue <id>` → 创建下一个 artifact
- `/llman-sdd:ff <id>` → 快速创建所有 artifacts
- `/llman-sdd:apply <id>` → 按 tasks 实施并更新 checkbox
- `/llman-sdd:verify <id>` → 核对实现与 artifacts 是否一致
- `/llman-sdd:archive <id>` → 合并 deltas 并移动到 `llmanspec/changes/archive/`

故障排查：
- Claude `/llman-sdd:*` 不生效：重新运行 `llman sdd update-skills --all`。
- 存在旧版命令目录或旧版 Codex prompts 绑定：在交互式终端运行 `llman sdd update-skills` 进行迁移（需要二次确认）。
<!-- endregion -->

<!-- region: validation-hints -->
校验修复最小示例：

1) 缺少 `## Purpose` 或 `## Requirements`：
```markdown
## Purpose
<用一句话说明目标>

## Requirements
### Requirement: <name>
The system MUST ...

#### Scenario: <happy path>
- **WHEN** ...
- **THEN** ...
```

2) 场景标题格式：
```markdown
#### Scenario: <name>
- **WHEN** ...
- **THEN** ...
```

3) 无 delta 变更：至少在
`llmanspec/changes/<change-id>/specs/<capability>/spec.md` 添加一个需求块：
```markdown
## ADDED Requirements
### Requirement: <name>
The system MUST ...

#### Scenario: <name>
- **WHEN** ...
- **THEN** ...
```
<!-- endregion -->

<!-- region: structured-protocol -->
## Context
- 执行前先确认当前 change/spec 状态。

## Goal
- 明确本次命令/skill 要达成的可验证结果。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。

## Workflow
- 以 `llman sdd` 命令结果为事实来源。
- 涉及文件/规范变更时执行校验。

## Decision Policy
- 高影响歧义必须先澄清。
- 已知校验错误下禁止强行继续。

## Output Contract
- 汇总已执行动作。
- 给出结果路径与校验状态。
<!-- endregion -->

<!-- region: future-planning -->
## Future 到执行的规划
- 将 `llmanspec/changes/<id>/future.md` 视为候选待办池，而不是静态备注。
- 审查 `Deferred Items`、`Branch Options`、`Triggers to Reopen`，并把每项归类为：
  - `now`（需要立即转化为可执行工作）
  - `later`（保留在 future.md，补充明确触发信号）
  - `drop`（移除或标记拒绝并说明原因）
- 对每个 `now` 项，产出明确落地路径：
  - 后续 change id（`add-...`、`update-...`、`refactor-...`）
  - 受影响 capability/spec 路径
  - 第一条可执行动作（`/llman-sdd:new`、`/llman-sdd:continue`、`/llman-sdd:ff` 或 `llman-sdd-apply`）
- 保持可追溯性：在新 proposal/design/tasks 中引用来源 future 条目。
- 若存在高不确定性，先暂停并提问，再创建新变更工件。
<!-- endregion -->

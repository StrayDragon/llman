<!-- llman-template-version: 1 -->
<!-- region: sdd-commands -->
常用命令：
- `llman sdd list`（列出变更）
- `llman sdd list --specs`（列出 specs）
- `llman sdd show <id>`（查看 change/spec）
- `llman sdd validate <id>`（校验变更或 spec）
- `llman sdd validate --all`（批量校验）
- `llman sdd archive <id>`（归档变更）
<!-- endregion -->

<!-- region: opsx-quickstart -->
OPSX 工作流：
- 安装/刷新：`llman sdd update-skills --all`
- Claude Code 命令绑定位置：`.claude/commands/opsx/`
- Codex 不生成 OPSX slash-command/custom-prompt 绑定；请使用 `llman-sdd-*` skills。

常见动作：
- `/opsx:new <id|description>` → 创建 `llmanspec/changes/<id>/`
- `/opsx:continue <id>` → 创建下一个 artifact
- `/opsx:ff <id>` → 快速创建所有 artifacts
- `/opsx:apply <id>` → 按 tasks 实施并更新 checkbox
- `/opsx:verify <id>` → 核对实现与 artifacts 是否一致
- `/opsx:archive <id>` → 合并 deltas 并移动到 `llmanspec/changes/archive/`

故障排查：
- Claude `/opsx:*` 不生效：重新运行 `llman sdd update-skills --all`。
- 存在 legacy 绑定（`.claude/commands/openspec/` 或 `.codex/prompts/openspec-*.md`）：在交互式终端运行 `llman sdd update-skills` 进行迁移（需要二次确认）。
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

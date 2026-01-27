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

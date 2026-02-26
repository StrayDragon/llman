<!-- llman-template-version: 1 -->
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

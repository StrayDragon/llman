# Design: add-skill-version-metadata

## 决策记录

### 1. 版本字段位置

**决策**：使用 Agent Skills 标准的 `metadata.version` 字段

**理由**：
- 符合 Agent Skills 标准，pi 等 harness 原生支持
- `metadata` 是标准定义的扩展点，用于存储额外属性
- 避免引入非标准字段

**替代方案**：
- 使用 `compatibility` 字段：语义不符，该字段用于环境要求而非版本
- 自定义 `llman-version` 字段：不符合标准，harness 可能忽略

### 2. 版本格式

**决策**：使用语义化版本（SemVer），如 `0.0.50`

**理由**：
- 与 llman CLI 的 `Cargo.toml` 版本一致
- 支持主版本/次版本/补丁版本的区分
- 便于后续实现主版本不匹配警告

### 3. 版本同步时机

**决策**：在 `llman sdd init` 和 `llman sdd update-skills` 时自动填充/更新

**理由**：
- 这两个命令是 skills 生命周期的主要入口
- 自动填充减少用户手动维护负担
- 更新时同步确保版本与当前 CLI 一致

**替代方案**：
- 仅在 init 时填充：用户可能忘记手动更新
- 要求用户手动填写：容易遗漏或出错

### 4. 版本不匹配处理

**决策**：主版本不匹配时输出警告，但不阻断执行

**理由**：
- 警告提供信息但不强制，保持向后兼容
- 主版本变更通常意味着重大行为变化
- 次版本/补丁版本不匹配不警告，减少噪音

**替代方案**：
- 阻断执行：过于严格，可能阻碍正常工作流
- 不警告：用户可能不知道 skills 已过时

## 技术方案

### 数据流

```
llman CLI (Cargo.toml version)
    │
    ├─► llman sdd init
    │   └─► 写入 SKILL.md: metadata.version = CLI version
    │
    ├─► llman sdd update-skills
    │   └─► 更新 SKILL.md: metadata.version = CLI version
    │
    └─► skill 加载时
        └─► 比较 metadata.version vs CLI version
            └─► 主版本不匹配 → 输出警告
```

### 文件变更

1. **模板文件**：增加 `metadata.version` 占位符
2. **init 模块**：读取 CLI 版本并写入模板
3. **update-skills 模块**：读取 CLI 版本并更新现有 SKILL.md
4. **skills 加载模块**：解析 `metadata.version` 并比较

## 开放问题

1. 是否需要在 `llman sdd list` 中显示 skills 版本？
2. 是否需要 `llman sdd upgrade-skills` 命令来批量更新版本？
3. 版本字段是否应该在 `.agents/skills/` 的模板中，还是仅在生成后的实际文件中？

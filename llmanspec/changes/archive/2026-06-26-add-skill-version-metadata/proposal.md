# Proposal: add-skill-version-metadata

## Why

当前 llman SDD skills 的 SKILL.md 模板没有版本概念。这导致：

1. **无法发现版本差异**：当 llman 从 v0.0.50 升级到 v1.0.0 时，skills 内容可能需要变化（新命令、新工作流），但没有机制标记 skills 适用的版本
2. **harness 无法感知兼容性**：pi 等 harness 按照 Agent Skills 标准加载 skills，但无法知道当前 skills 是否与安装的 llman 版本兼容
3. **升级时缺乏提示**：用户升级 llman 后，旧 skills 可能指引过时的命令或流程，导致困惑

## What Changes

利用 Agent Skills 标准中的 `metadata` 字段，为 skills 增加版本信息：

1. **SKILL.md 模板增加 `metadata.version`**：
   ```yaml
   metadata:
     version: "0.0.50"  # 与 llman CLI 版本同步
   ```

2. **skills 生成/更新时自动填充版本**：
   - `llman sdd init` 生成 skills 时写入当前 CLI 版本
   - `llman sdd update-skills` 更新时同步版本

3. **可选：增加版本检查步骤**：
   - 在 skill 执行时检查 `metadata.version` 与当前 CLI 版本是否匹配
   - 版本不匹配时给出警告

## Capabilities

- `skills-management`：现有 spec，描述技能发现、托管、链接的整体流程

## Impact

- **正向**：用户和 harness 可以识别 skills 版本，升级时有明确提示
- **风险**：低。仅增加 metadata 字段，不改变现有行为
- **迁移**：现有 skills 无 `metadata.version` 时，视为"未版本化"，不阻断使用

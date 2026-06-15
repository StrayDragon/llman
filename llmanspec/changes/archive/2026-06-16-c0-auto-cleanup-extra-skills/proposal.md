---
depends_on: []
blocks: []
---

# 自动清理不再需要的 extra_skills 技能文件

## Why

当前 `llman sdd init --update` 命令在更新技能文件时，只负责**写入**新技能，但**不会删除**已存在但不再在 `extra_skills` 配置中的技能文件。这导致：

1. **手动清理负担**：用户减少 `extra_skills` 配置后，需要手动删除 `.agents/skills/` 目录下对应的技能文件夹。
2. **配置与实际不一致**：配置文件显示某些技能已禁用，但 `.agents/skills/` 目录中仍然存在这些技能文件，可能造成混淆。
3. **违反声明式原则**：配置应该是"声明式"的——配置文件描述期望状态，工具负责使实际状态与期望状态一致。

## What Changes

- **自动清理逻辑**：`llman sdd init --update` 在写入技能文件前，先扫描 `.agents/skills/` 目录中已存在的技能，与配置中期望的技能列表对比，删除不再需要的技能目录。
- **安全保护**：只删除 `OPTIONAL_SKILL_FILES` 中定义的可选技能目录，不会误删用户自定义的其他技能。
- **日志提示**：删除技能时输出 INFO 级别日志，告知用户哪些技能被清理。

## Capabilities

- sdd-workflow（MODIFY update-skills 自动清理逻辑）

## Impact

- **CLI 行为**：`llman sdd init --update` 将自动清理不再需要的技能目录，用户无需手动删除。
- **向后兼容**：不影响现有功能，只是增加了自动清理能力。
- **安全性**：只清理可选技能，不会误删核心技能或用户自定义技能。

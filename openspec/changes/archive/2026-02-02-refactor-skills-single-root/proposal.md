## Why
当前 skills 管理围绕“多来源扫描 + 托管快照 store/ + 版本冲突选择”设计，但使用者已经把 `~/.config/llman/skills/` 作为唯一受控的技能仓库（git 管理）。现有流程不仅引入额外复制与占用，还要求处理来源/快照/冲突，导致交互复杂、目标端经常因非 symlink 报错；同时技能目录与 `SKILL.md` 常通过软链接组织，当前扫描会漏掉这些技能。

## What Changes
- **BREAKING** skills 管理改为单一来源：仅扫描 `<skills_root>`，不再读取 `[[source]]` 或 repo scope。
- **BREAKING** 移除 `store/` 快照与版本冲突选择；目标链接直接指向 `<skills_root>/<skill_id>`。
- **BREAKING** 移除 `copy` 模式，仅保留 `link/skip`。
- `config.toml` 升级为 v2，仅保留 targets（`mode` 默认 `link`）。
- 扫描允许技能目录或 `SKILL.md` 为软链接，只要目标解析后包含 `SKILL.md` 即视为技能。
- 交互流程重构：多选技能 → 为每个技能选择目标 → 确认后同步；发现目标冲突时提示是否覆盖（删除重建）。

## Impact
- Affected specs: `skills-management`
- Affected code: `src/skills/*`, config schema/CLI prompts/tests
- Migration: 需要将 `store/<skill>/current` 内容迁移到 `<skills_root>/<skill_id>` 并更新 `config.toml`

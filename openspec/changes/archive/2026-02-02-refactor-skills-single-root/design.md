## Context
当前 skills 管理器会从多个来源扫描技能并复制到 `store/`，再通过 `registry.json` 选择 `current` 版本并为目标创建链接/副本。用户已经把 `~/.config/llman/skills/` 作为唯一技能仓库（git 管理），希望 llman 只管理这一个目录，避免“窃取/快照”逻辑和额外占用。

## Goals / Non-Goals
- Goals:
  - 以 `<skills_root>` 作为唯一来源，直接分发
  - 精简交互流程：选择技能 → 选择目标 → 应用
  - 仅支持 link/skip（默认 link）并在冲突时提示覆盖
  - 让用户用 git 自行控制版本历史
- Non-Goals:
  - 自动拉取/更新远端仓库或子模块
  - 保留多来源冲突选择与历史快照
  - 兼容旧 `config.toml` v1 格式

## Decisions
1) **单一来源**
- `<skills_root>` 即技能来源，不再读取 `[[source]]` 或 repo scope。
- `config.toml` 升级为 v2，仅包含 `[[target]]`（`mode` 缺省为 `link`）。

2) **移除快照仓库**
- 不再使用 `store/` 与 `current` 版本链接。
- `registry.json` 仅用于记录每个技能在各 target 的启用状态与上次同步时间（保留文件位置）。

3) **目标同步策略**
- `mode=link`: 创建 `target/<skill_id> -> <skills_root>/<skill_id>`。
- `mode=skip`: 目标跳过且在交互中只读展示。
- 目标存在且不满足预期时提示覆盖（删除后重建）。

4) **交互式流程**
- 进入管理器后先多选技能（可过滤）。
- 对每个技能展示 target 列表（默认值来自 registry/config），用户勾选确认后同步。

5) **扫描规则**
- 扫描 `<skills_root>` 下的 `SKILL.md`，遵循 `.gitignore` / 全局忽略规则。
- 允许技能目录或 `SKILL.md` 为软链接；解析目标后若包含 `SKILL.md` 即视为技能。
- 扫描时维护已访问的 canonical 路径，避免软链接循环。

## Risks / Trade-offs
- 失去 `store/` 快照与历史版本，转而依赖 git 历史；需要用户自行维护。
- 移除多来源扫描后无法自动合并外部目录；必须由用户手动引入到 `<skills_root>`。
- 允许软链接扫描后可能指向仓库外路径；通过循环检测避免无限遍历，但仍需用户自控。

## Migration Plan
1) 备份或保留现有 `store/` 目录。
2) 对每个技能，将 `store/<skill_id>/current` 内容复制到 `<skills_root>/<skill_id>`（若尚不存在）。
3) 将 `config.toml` 迁移到 v2，仅保留 targets。
4) 运行新的 `llman skills` 以重建 `registry.json`。

## Rollback Plan
- 继续保留旧二进制或回滚变更。
- 恢复旧 `config.toml` v1 与 `store/`，重新运行旧版 `llman skills`。

## Open Questions
- `registry.json` 是否需要显式 `version` 字段用于未来迁移？
- 非交互模式的冲突默认策略是否必须强制 `--target-conflict`？

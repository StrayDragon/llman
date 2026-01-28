## Why
当前 `llman skills` 会重链接来源目录并替换为软链接，用户体验上容易被视作“窃取”，且与由 Claude Code 插件管理的路径存在冲突风险。同时缺少安全的复制模式与明确的冲突策略，非交互场景也缺少可预测的行为入口。

## What Changes
- 来源扫描改为只读：托管快照仍生成，但不重链接来源目录。
- 移除 `--relink-sources` 参数（破坏性变更）。
- 目标支持 `mode = link|copy|skip`（默认 link），copy 通过覆盖复制更新，skip 永不更新。
- copy 模式写入 `.llman-skill.json` 用于检测本地改动；冲突仅提供 overwrite/skip。
- 交互模式出现冲突时提示选择；非交互必须传入 `--target-conflict=overwrite|skip`，否则报错并提示。
- `llman skills` 允许无 `--relink-sources` 执行只读同步并进入管理器。

## Impact
- specs: skills-management
- code: src/skills/{command,config,types,sync,scan}、locales/app.yml

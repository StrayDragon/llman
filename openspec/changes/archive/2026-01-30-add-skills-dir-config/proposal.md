## Why
当前技能管理的根目录固定为 `LLMAN_CONFIG_DIR/skills`，无法像 `/home/.../llman.skills` 这样的独立路径进行管理。需要支持可配置的 skills 根目录，并允许通过 CLI/环境变量/llman 配置文件覆盖，同时保持默认行为不变。

## What Changes
- 新增 skills 根目录解析优先级：`--skills-dir` > `LLMAN_SKILLS_DIR` > llman 配置（`config.yaml` 的 `skills.dir`，本地优先）> 默认 `LLMAN_CONFIG_DIR/skills`。
- 扩展 llman 配置文件，新增可选 `skills` 章节（仅用于 skills 根目录配置）。
- 技能管理改为从解析出的 skills 根目录下读取/写入 `config.toml`、`registry.json`、`store/`。
- 更新相关 CLI 帮助、错误信息与测试覆盖。

## Impact
- Specs: `skills-management`（路径解析与默认行为）
- Code: `src/skills/command.rs`, `src/skills/config.rs`（路径解析与配置加载）
- Config schema: `LLMAN_CONFIG_DIR/config.yaml`（新增 `skills.dir`）
- Tests: skills 配置路径解析与优先级测试

## Context
当前技能同步会将来源目录替换为指向托管仓库的软链接，导致来源被“接管”。这与插件管理器对技能目录的管理方式相冲突，也不利于用户在来源路径保留原状。我们需要提供只读来源同步，并允许目标路径采用链接或复制，同时提供明确的冲突策略。

## Goals / Non-Goals
- Goals:
  - 来源目录只读；不重链接、不替换来源内容。
  - 目标路径支持 link/copy/skip 三种模式，默认 link。
  - copy 模式具备冲突检测与最小化误覆盖的安全策略。
  - `llman skills` 无需 `--relink-sources` 即可运行并进入管理器。
- Non-Goals:
  - 不自动扫描或加入 `~/.claude/plugins/marketplaces` 等插件路径作为默认来源/目标。
  - 不改动默认来源/目标路径集合。
  - 不引入新的包管理或远程下载机制。

## Decisions
- 来源目录始终只读：同步阶段只负责扫描与托管快照，不修改来源目录结构或软链接。
- 目标配置新增 `mode`：`link`（默认）、`copy`、`skip`。
  - `skip` 目标在同步与管理器中都被跳过，用于插件管理路径。
- copy 模式写入 `.llman-skill.json`：记录 `skill_id`、`hash`、`updated_at`、`last_written_hash` 等，用于检测本地修改。
  - 目标存在但无 metadata，或 metadata 与当前内容不一致，视为冲突。
- 冲突策略：交互模式仅提供 overwrite/skip；非交互必须传入 `--target-conflict=overwrite|skip`，否则报错并提示。
- `llman skills` 默认运行只读同步并进入管理器；移除 `--relink-sources` 参数以避免误导。

## Alternatives considered
- 保留来源重链接：拒绝（用户体验差且与插件管理冲突）。
- 目标默认 copy：拒绝（磁盘占用与冲突成本更高）。
- 自动扫描插件 marketplace 路径：拒绝（路径结构不稳定且与默认路径目标不一致）。

## Risks / Trade-offs
- copy 模式增加磁盘占用与更新成本。
- 需要明确的冲突策略以避免覆盖用户手改内容。
- `skip` 目标不会被管理器控制，需在配置层面告知用户。

## Migration Plan
- 现有配置不变；`mode` 缺省为 `link`。
- `--relink-sources` 参数删除；用户需改为直接运行 `llman skills` 或使用新的冲突参数。
- 若用户需要避免更新插件路径，可在 `config.toml` 中为目标设置 `mode = "skip"`。
- registry 结构保持不变；copy 目标新增 `.llman-skill.json` 作为本地标记文件。

## Open Questions
- 暂无

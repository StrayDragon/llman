## 为什么
用户在多个 agent（Claude、Codex、通用 Agent Skills）之间保存 skills，目录与 scope 不一致，导致重复、漂移和手工维护成本。通过一个交互式管理器集中存储与分发，可以降低摩擦并保留版本历史。

## 变更内容
- 新增 `llman skills` 作为交互式技能管理入口（v1 不提供非交互子命令）。
- 在 `LLMAN_CONFIG_DIR/skills` 下引入托管技能仓库，并以内容哈希进行快照记录。
- 进入命令时扫描已配置的技能来源，导入未托管技能，并用指向托管仓库的软链接替换来源目录。
- 提供交互式冲突解决、按 agent 启用/禁用，以及删除等操作。
- 在 `LLMAN_CONFIG_DIR/skills` 下增加配置文件，用于定义来源/目标与默认值。

## 影响
- 新增规范：`skills-management`。
- 新增 CLI 流程与本地化用户提示。
- 在 `LLMAN_CONFIG_DIR/skills` 下新增配置/状态文件。
- 新增发现、哈希与软链接行为测试。

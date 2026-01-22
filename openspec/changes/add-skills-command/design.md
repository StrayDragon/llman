## 背景
`llman` 已通过 `LLMAN_CONFIG_DIR` 集中配置并提供交互式代理工具流程。当前 skills 分散在多个 agent 和 scope 下，用户缺乏统一管理版本、启用状态与冲突的方式。

## 目标 / 非目标
目标：
- 在 `LLMAN_CONFIG_DIR/skills` 下集中管理 skills。
- 自动发现并导入常见 agent 位置的现有 skills。
- 通过内容哈希（md5）保留所有版本快照。
- 提供交互式管理器用于冲突处理与按 agent 启用/禁用。

非目标：
- 不管理非技能类资产（例如 Cursor `.cursor/rules`）。
- v1 不新增非交互子命令。
- v1 不实现 GitHub 远程安装。

## 决策
1. 托管仓库为唯一真相。
   - 外部来源用指向托管仓库的软链接替换。
2. 进入 `llman skills` 时先自动同步。
   - 未托管 skills 自动导入并重链接。
   - 有冲突先提示用户再重链接。
3. 版本标识使用整个技能目录的 md5。
   - 新哈希生成新快照；已存在哈希复用。
4. 冲突默认排序（用于推荐选择）：
   - Claude > Codex > Agent Skills。
   - Codex 内部：repo scope > user scope > admin scope。
5. v1 不管理 Cursor。
   - 依赖 Cursor 自行导入 Claude Code skills。

## 存储结构
`LLMAN_CONFIG_DIR/skills` 下的建议结构：

```
skills/
  config.toml
  registry.json
  store/
    <skill-name>/
      current/                # 指向 versions/<hash> 的软链接
      versions/<hash>/        # 不可变快照
```

- `config.toml` 定义来源与目标（路径、scope、启用状态）。
- `registry.json` 记录托管技能、哈希、来源历史与时间戳。

## 默认来源与目标
在 `config.toml` 缺失时使用默认值，可配置覆盖。

来源（目录存在时生效）：
- Claude：`$CLAUDE_HOME/skills` 或 `~/.claude/skills`。
- Codex（repo scope）：`$CWD/.codex/skills`、`$CWD/../.codex/skills`、`$REPO_ROOT/.codex/skills`。
- Codex（user scope）：`$CODEX_HOME/skills` 或 `~/.codex/skills`。
- Codex（admin scope）：`/etc/codex/skills`（无权限时只读）。
- Agent Skills（通用）：`~/.skills`（可配置）。

目标（启用/禁用落点）：
- 与来源相同，按 agent 分组。

## 自动同步流程
1. 发现技能目录（包含 `SKILL.md` 的目录）。
2. 对每个技能：
   - 以确定性遍历计算 md5 哈希。
   - 若哈希为新值，则快照至 `store/<skill>/versions/<hash>` 并更新 registry。
   - 确保 `store/<skill>/current` 指向所选版本。
3. 将来源目录替换为指向 `store/<skill>/current` 的软链接。
   - 若来源已是指向仓库的软链接则跳过。
   - 若创建软链接失败（权限、占用等），给出提示并继续。

## 冲突解决
当多个来源提供同名但哈希不同的技能时：
- 提示用户选择激活版本。
- 默认选择遵循上述优先级排序。
- 未选版本保留在 `versions/` 中并记录于 registry。
- 选择完成后，所有目标链接指向所选版本。

## 启用/禁用语义
- 启用：在目标 agent 目录下创建指向托管技能的软链接。
- 禁用：仅移除该目标目录下的软链接，保留托管副本。

## 未决问题
- 无。

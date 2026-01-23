## 背景
`llman` 已通过 `LLMAN_CONFIG_DIR` 集中配置并提供交互式代理工具流程。当前 skills 分散在多个 agent 和 scope 下，用户缺乏统一管理版本、启用状态与冲突的方式。

## 目标 / 非目标
目标：
- 在 `LLMAN_CONFIG_DIR/skills` 下集中管理 skills。
- 自动发现并导入常见 agent 位置的现有 skills。
- 通过内容哈希（md5）保留所有版本快照。
- 提供交互式管理器用于冲突处理与按 agent 启用/禁用。
- 遵循 `.gitignore`/全局忽略规则并跳过软链接，避免管理自动生成内容。
- 项目级路径自动发现（基于 git 根目录），无需配置显式声明。

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
   - Codex 内部：repo scope > user scope。
5. 配置采用结构化列表（schema versioned），仅声明 global/user 级别来源与目标。
6. repo/project scope 自动发现。
   - 运行时在 git 根目录下探测 `.claude/skills` 与 `.codex/skills`。
   - 不在配置中声明 repo 路径。
7. 扫描遵循 `.gitignore` 与全局忽略规则，跳过软链接与被忽略条目。
8. skill ID 规则：优先使用 `SKILL.md` frontmatter `name` 的 slug（小写、非字母数字替换为 `-`、去除首尾 `-`、最多 64 个字符），非法或缺失时回退目录名。
9. v1 不管理 Cursor。
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

- `config.toml` 定义来源与目标（路径、scope、启用状态、version）。
- `registry.json` 记录托管技能、哈希、时间戳与启用状态（必要时包含来源历史）。

最小 registry 结构示例：
```json
{
  "skills": {
    "agent-skills-expert": {
      "current_hash": "md5",
      "versions": {
        "md5": {
          "created_at": "2025-01-05T15:37:00Z"
        }
      },
      "targets": {
        "claude_user": true,
        "codex_user": true
      }
    }
  }
}
```

## 默认来源与目标
在 `config.toml` 缺失时使用默认值，可配置覆盖。仅包含 global/user 级别来源与目标。

来源（目录存在时生效）：
- Claude：`$CLAUDE_HOME/skills` 或 `~/.claude/skills`。
- Codex（user scope）：`$CODEX_HOME/skills` 或 `~/.codex/skills`。
- Agent Skills（通用）：`~/.skills`（可配置）。

目标（启用/禁用落点）：
- 与来源相同，按 agent 分组。

示例配置（schema B）：

```toml
version = 1

[[source]]
id = "claude_user"
agent = "claude"
scope = "user"
path = "~/.claude/skills"
enabled = true

[[source]]
id = "codex_user"
agent = "codex"
scope = "user"
path = "~/.codex/skills"
enabled = true

[[source]]
id = "agent_global"
agent = "agent"
scope = "global"
path = "~/.skills"
enabled = true

[[target]]
id = "claude_user"
agent = "claude"
scope = "user"
path = "~/.claude/skills"
enabled = true
```

## 项目级自动发现
- 若当前工作目录位于 git 仓库内，自动探测仓库根目录下的 `.claude/skills` 与 `.codex/skills` 作为 repo scope 来源/目标。
- 若不在 git 仓库内，提示用户确认是否继续扫描 global/user 来源。

## 自动同步流程
1. 解析配置与默认来源/目标，必要时进行 git 根目录探测。
2. 构建来源列表（global/user + repo auto-discover）。
3. 发现技能目录（包含 `SKILL.md` 的目录），遵循 `.gitignore`/全局忽略规则并跳过软链接。
4. 对每个技能：
   - 以确定性遍历计算 md5 哈希（仅真实文件，排除软链接与被忽略条目）。
   - 若哈希为新值，则快照至 `store/<skill>/versions/<hash>` 并更新 registry。
   - 确保 `store/<skill>/current` 指向所选版本。
5. 将来源目录替换为指向 `store/<skill>/current` 的软链接。
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

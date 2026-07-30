---
depends_on: []
branch: sdd/add-multi-skill-repo-source-support-with-tui-filtering
base_sha: da7c8179667ddbddb70436b02648e739024dd290
checkpointed: true
checkpoint_sha: da7c8179667ddbddb70436b02648e739024dd290
---

# 多技能仓库源支持与 TUI 按来源筛选

## Why

用户目前只能有一个 skills 根目录（`~/.config/llman/skills/`），通过 `LLMAN_SKILLS_DIR`、
`--skills-dir` 或 `skills.dir` 配置。所有技能都平铺在这个单一目录下，无法区分技能来自哪个
仓库源。

社区和组织有多个技能仓库需要隔离管理：
- 不同团队维护独立的 skill repo
- 个人技能库 vs 组织技能库需要分开
- 项目特有的技能源与全局技能源分离

TUI 侧也缺少按来源筛选/分组的能力——目前的 preset 分组基于目录名的 `.` 前缀（如
`dakesan.*`），而非显式的 repo 来源元数据。

## What Changes

### 配置层：扩展 `skills` 段，以 `repo` 列表替代单 `dir`

现有单目录配置：

```yaml
skills:
  dir: $LLMAN_CONFIG_DIR/skills
```

扩展为支持多仓库源：

```yaml
skills:
  repo:
    - path: $LLMAN_CONFIG_DIR/skills    # name 省略时默认用相对短路径
    - name: 团队技能库                    # 可选，用于 TUI 显示
      path: /path/to/team-skills
```

- `name` 可选，省略时 fallback 为 `path` 的相对短路径名（最后一个路径分量或相对于
  `$LLMAN_CONFIG_DIR` 的简写）。
- 向后兼容：检测到旧的 `skills.dir` 字段时，自动转为单 repo `[{path: <原值>}]`。
- 向后兼容：`skills.dir` 与 `skills.repo` 同时存在时，`repo` 优先（`dir` 被忽略，emit
  deprecation warning）。
- 未来考虑：`path` 支持 git URL（当前仅本地路径，clone/update 机制不在本变更范围内）。

### Override 语义（决策）

`--skills-dir` / `LLMAN_SKILLS_DIR` 在多 repo 场景下保持**全盘替代**语义：override 存在
时，整个 repo 列表被该单一目录替代（即变成 `[{path: <override>}]`）。这与现有单目录
override 优先级链 `CLI > ENV > config > default` 完全一致，迁移无惊讶。

### 跨 repo 同名 skill_id 去重（决策）

多个 repo 下出现相同 `skill_id` 时，**按 repo 在配置列表中的顺序，首个生效，其余记录
冲突警告**（复用现有 `dedupe_skills` 的冲突提示通道）。与现有 r34 单源去重行为一致。

### TUI picker 分组改造

现有 TUI picker 的分组逻辑是**解析目录名前缀**（`dakesan.*` → 归入 `dakesan` 组）。
变更后改为：

- **默认分组维度改为 repo 源**：每个 `skills.repo[]` 成为一个折叠/展开组，组名显示
  `name`（若有）或路径简称。
- **保留目录名前缀分组作为第二维度**：展开 repo 组后，子技能仍可按 `组织.技能名` 前缀次级
  分组。
- `name` 会在 TUI 中作为 repo 来源标签显示在技能行旁（当存在多 repo 时）。
- `/` 搜索扩展到匹配 `name` / 路径简称。

### 数据模型：技能引入 repo 来源元数据

- `SkillCandidate` 增加 `repo_id: Option<String>` / `repo_name: Option<String>` 字段。
- 技能发现（`discover_skills`）在扫描时记录来自哪个 repo 条目。

### 范围

- 仅影响**用户侧技能仓库引入**——不影响 llman 内置 SDD skills（`.agents/skills/`）的发现
  与安装机制。
- 不引入 git clone/update 能力（仅本地路径引用）。
- `--skills-dir` CLI flag 和 `LLMAN_SKILLS_DIR` 环境变量保持不变，仍可覆盖整个 skills 根
  （当有 `skills.repo` 配置时，override 行为为**全盘替代** repo 列表，见上文 Override 语义）。

## Capabilities

- `config-schemas`（r125）：全局配置 schema 新增 `skills.repo[]`（`name`/`path`）；
  `skills.dir` 向后兼容读取与 deprecation warning；`repo` 优先于 `dir`。属 CLI 驱动的
  可执行合约（`llman self schema check` 可触发）。
- `skills-management`（r126）：技能发现按 repo 记录来源元数据；TUI 默认分组维度改为 repo
  源、目录名前缀降为次级维度；搜索匹配 `name`/路径简称；跨 repo 同名按列表顺序首个生效。

## Impact

- **受影响代码**：`src/skills/config/mod.rs`（多 repo 解析 + override 全盘替代）、
  `src/skills/catalog/scan.rs`（携带 repo 元数据）、`src/skills/catalog/types.rs`
  （`SkillCandidate` 增字段）、`src/skills/cli/command.rs`（TUI 分组改造）、
  `src/config_schema.rs`（`GlobalSkillsConfig` schema 扩展）。
- **向后兼容**：现有 `skills.dir` 配置零改动可用（自动转单 repo）。
- **full mode 判定**：`config-schemas`（r125）场景可由 CLI 子进程驱动，标 `@executable`；
  `skills-management`（r126）的 TUI 分组为内部行为 + 交互流程，保持 fast mode（单元测试
  覆盖），不标 `@executable`。

---
depends_on: []
---

## Why

用户目前只能有一个 skills 根目录（`~/.config/llman/skills/`），通过 `LLMAN_SKILLS_DIR`、
`--skills-dir` 或 `skills.dir` 配置。所有技能都平铺在这个单一日录下，无法区分技能来自哪个
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
- 向后兼容：`skills.dir` 与 `skills.repo` 同时存在时，`repo` 优先（`dir` 被忽略，可 emit
  deprecation warning）。
- 未来考虑：`path` 支持 git URL（当前仅本地路径，clone/update 机制不在本变更范围内）。

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
  （当有 `skills.repo` 配置时，CLI/env 覆盖行为需明确：是全盘替代 repo 列表，还是仅追加？
  待 propose 阶段决定）。

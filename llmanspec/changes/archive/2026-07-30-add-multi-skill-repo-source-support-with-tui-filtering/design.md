# Design: 多技能仓库源支持与 TUI 按来源筛选

## 上下文与约束

当前 skills 来源是**单一根目录**，优先级链为 `CLI --skills-dir` > `LLMAN_SKILLS_DIR` >
`config.yaml skills.dir` > 默认 `$LLMAN_CONFIG_DIR/skills`（见 `src/skills/config/mod.rs`
`resolve_skills_root` / `resolve_skills_root_with`）。override 始终是「整个 skills 根」的
全盘覆盖。

TUI 分组（`src/skills/cli/command.rs`）当前**解析目录名 `.` 前缀**推断分组
（`infer_skill_group_from_dir`），无 repo 来源概念。

约束：
- 不破坏现有 `skills.dir` 配置（零改动可用）。
- 不引入 git clone/update（仅本地路径引用）。
- 交互式 inquire 流程不写自动化测试（AGENTS.md 规则）；TUI 分组用单元测试覆盖。
- 不影响 llman 内置 SDD skills（`.agents/skills/`）发现机制。

## 决策

### D1: override 语义 = 全盘替代

`--skills-dir` / `LLMAN_SKILLS_DIR` 在多 repo 场景下，override 存在时**整个 repo 列表被
该单一目录替代**（等价于 `[{path: <override>}]`）。

**理由**：与现有单目录 override 语义一致，最小惊讶。不引入「override 作为第 N+1 个 repo
追加」的混合语义——那会让「为什么设了 env 后 repo 列表还在」变得不可预测。

**实现落点**：`resolve_skills_root` 系列函数升级为返回 `Vec<RepoSource>` 而非单个
`PathBuf`；override 分支返回单元素列表。

### D2: 跨 repo 同名 skill_id = 列表顺序首个生效 + 冲突警告

多个 repo 出现相同 `skill_id` 时，按 repo 在 `skills.repo[]` 中的顺序，**首个生效**，
其余通过现有 `dedupe_skills` 的冲突提示通道（`skills.manager.duplicate_skill_id` i18n key）
记录警告。

**理由**：与现有 r34 单源「重叠技能去重」行为一致；可预测、可诊断；不强制用户中断。

**实现落点**：`dedupe_skills` 已按 `skill_id` 去重，只需让多 repo 扫描结果按 repo 顺序
串接，现有去重逻辑天然保证「首个生效」。

### D3: 分组维度分层——repo 为顶层，目录名前缀为次级

TUI 默认分组改为 repo 源（每个 `skills.repo[]` 一个折叠组，组名 = `name` 或路径简称）。
展开 repo 组后，子技能仍按 `组织.技能名` 前缀次级分组（保留现有 `.` 前缀逻辑作为第二维）。

**理由**：repo 是用户显式声明的、更稳定的分组维度；目录名前缀是隐式约定，降为次级可
向后兼容现有 `dakesan.*` 习惯。

**实现落点**：`grouped_skill_options` 改造为两层；`SkillCandidate` 携带
`repo_id`/`repo_name`（来自 `discover_skills` 扫描时记录）。

### D4: 单 repo 时退化——不显示 repo 分组头

仅配置 1 个 repo（含 `skills.dir` 自动转换的情况）时，TUI 行为**与现状完全一致**（不显示
repo 分组头，仅按目录名前缀分组），避免对单源用户引入噪声。

## 数据模型变更

```rust
// src/skills/catalog/types.rs
pub struct SkillCandidate {
    pub skill_id: String,
    pub skill_dir: PathBuf,
    pub repo_id: Option<String>,    // 新增：来自哪个 repo（单源时为 None）
    pub repo_name: Option<String>,  // 新增：repo 显示名
}

// 新增：repo 源描述
pub struct RepoSource {
    pub id: String,          // 稳定标识（列表索引或 name）
    pub name: Option<String>,
    pub path: PathBuf,
}
```

`SkillsPaths` 演进：保留 `root`（向后兼容，取首个 repo）+ 新增 `repos: Vec<RepoSource>`。

## 迁移与向后兼容

1. 旧 `skills.dir: X` → 解析为 `repos: [RepoSource{ path: X, name: None }]`，行为不变。
2. `skills.dir` 与 `skills.repo` 同存 → `repo` 优先，`dir` 忽略 + emit deprecation warning
   （stderr，不阻断）。
3. `--skills-dir` / `LLMAN_SKILLS_DIR` 存在 → 全盘替代为单元素 repo 列表（D1）。
4. config schema（`GlobalSkillsConfig`）新增 `repo: Option<Vec<RepoEntrySchema>>`，`dir`
   字段保留（标 deprecated 描述）。

## 风险与权衡

- **风险**：`SkillsPaths` 从单 root 改为多 repo 是较深的签名变更，会波及 `load_config`、
  `ensure_dirs`、`discover_skills` 调用链。**缓解**：按垂直切片推进（见 tasks），每片可独立
  编译 + 测试。
- **权衡**：未引入 git URL 支持（明确排除出范围），`path` 仅本地路径——避免本变更膨胀。
- **full mode 边界**：`config-schemas`（r125）可 CLI 驱动（`llman self schema check`），
  标 `@executable`；`skills-management`（r126）TUI 分组为内部行为，保持 fast mode。

## 测试边界（seam）

| Seam | 驱动 | 覆盖方式 |
|---|---|---|
| 多 repo schema 校验 / 向后兼容 / deprecation | CLI `llman self schema check` + config fixture | r125 `.feature` `@executable`，复用 `运行 llman {args}` / `退出码为` / `stderr 包含` |
| TUI 分组 / 跨 repo 去重 / 搜索匹配 | public fn `grouped_skill_options` / `dedupe_skills` / `discover_skills` | r126 单元测试（fast mode） |

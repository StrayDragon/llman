# Tasks: 多技能仓库源支持与 TUI 按来源筛选

> 垂直切片：每个 task 一刀切穿 数据模型 → 配置解析 → schema → CLI/TUI → 测试 的完整窄路径，
> 可独立编译与验证。依赖用 `[blocked-by]` 标注。

## T1: 数据模型引入 repo 来源元数据

- [ ] `src/skills/catalog/types.rs`：`SkillCandidate` 增 `repo_id: Option<String>` /
      `repo_name: Option<String>` 字段。
- [ ] 新增 `RepoSource { id, name: Option<String>, path: PathBuf }` 结构。
- [ ] `SkillsPaths` 增 `repos: Vec<RepoSource>` 字段（保留 `root` 取首个 repo 作向后兼容）。
- [ ] 更新所有构造 `SkillCandidate` / `SkillsPaths` 的调用点使其编译（暂填 None / 空）。
- [ ] 单元测试：`RepoSource` 构造与默认值。

## T2: 配置层多 repo 解析 + override 全盘替代（D1）

- [ ] `src/skills/config/mod.rs`：新增解析 `skills.repo[]`（`name`/`path`）为
      `Vec<RepoSource>`。
- [ ] 向后兼容：`skills.dir` 自动转单 repo 列表。
- [ ] `skills.dir` 与 `skills.repo` 同存 → `repo` 优先 + deprecation warning（stderr）。
- [ ] override（`--skills-dir` / `LLMAN_SKILLS_DIR`）→ 全盘替代为单元素 repo 列表（D1）。
- [ ] 更新 `resolve_skills_root*` 系列返回 repo 列表；`ensure_dirs` 处理多 repo。
- [ ] 单元测试：单 dir 兼容、repo 优先于 dir、override 全盘替代、name 缺失 fallback。
- [ ] `[blocked-by: T1]`

## T3: 技能发现携带 repo 元数据

- [ ] `src/skills/catalog/scan.rs`：`discover_skills` 接受 `&[RepoSource]`（或新签名），
      扫描每个 repo 时把 `repo_id`/`repo_name` 写入 `SkillCandidate`。
- [ ] 跨 repo 同名 `skill_id` 按列表顺序首个生效（D2），复用 `dedupe_skills` 冲突提示。
- [ ] 单元测试：多 repo 扫描结果携带正确 repo_id；跨 repo 同名去重 + 警告。
- [ ] `[blocked-by: T2]`

## T4: config schema 扩展（r125 可执行合约）

- [ ] `src/config_schema.rs`：`GlobalSkillsConfig` 增 `repo: Option<Vec<RepoEntrySchema>>`
      （`name: Option<String>` / `path: String`）；`dir` 字段描述标 deprecated。
- [ ] 更新 `Default for GlobalConfig`：默认仍输出单 `dir`（向后兼容）。
- [ ] 重生成 `artifacts/schema/configs/en/llman-config.schema.json`。
- [ ] 单元测试：schema 含 `repo` 定义；旧 `dir` 配置实例仍校验通过。
- [ ] `[blocked-by: T2]`

## T5: config-schemas 可执行 feature（r125 @executable）

- [ ] 在 `llmanspec/specs/config-schemas/` 新增/扩展 feature，标 `@req:r125` `@executable`：
      - 多 repo 配置通过 schema 校验
      - 旧 `skills.dir` 配置仍校验通过（向后兼容）
      - `dir`+`repo` 同存时 repo 优先 + deprecation warning（stderr）
- [ ] 复用泛化 step：`当 运行 llman self schema check ...` / `那么 退出码为` / `那么 stderr 包含`。
- [ ] `cargo test --features bdd` 该场景通过（full mode）。
- [ ] `[blocked-by: T4]`

## T6: TUI 分组分层——repo 顶层 + 目录名前缀次级（D3）

- [ ] `src/skills/cli/command.rs`：`grouped_skill_options` 改造为两层分组（repo → 目录名前缀）。
- [ ] 单 repo（含 `skills.dir` 自动转换）时退化为现状（D4，不显示 repo 分组头）。
- [ ] `name` / 路径简称作为 repo 组标签；多 repo 时技能行旁显示来源标签。
- [ ] `/` 搜索扩展匹配 `name` / 路径简称。
- [ ] 单元测试：多 repo 分层结构、单 repo 退化、搜索匹配 repo 名。
- [ ] `[blocked-by: T3]`

## T7: skills-management 不可执行 feature（r126 fast mode）

- [ ] 在 `llmanspec/specs/skills-management/` 新增 feature，标 `@req:r126`（**不标** `@executable`）：
      - 技能发现按 repo 记录来源元数据
      - 多 repo 下 TUI 默认按 repo 源分组（顶层）
      - 单 repo 时不显示 repo 分组头（向后兼容）
      - 跨 repo 同名按列表顺序首个生效 + 冲突警告
      - 搜索匹配 repo name / 路径简称
- [ ] 这些场景由单元测试覆盖（fast mode），不绑定 `#[scenario]`。
- [ ] `[blocked-by: T6]`

## T8: i18n + 回归校验

- [ ] 新增/更新 i18n key（deprecation warning、repo 分组标签等）——仅 en。
- [ ] `just check`（fmt + clippy + test）全绿。
- [ ] `llman sdd validate add-multi-skill-repo-source-support-with-tui-filtering --strict --no-check` 通过。
- [ ] `[blocked-by: T5, T7]`

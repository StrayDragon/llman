# Tasks: 多技能仓库源支持与 TUI 按来源筛选

> 垂直切片：每个 task 一刀切穿 数据模型 → 配置解析 → schema → CLI/TUI → 测试 的完整窄路径，
> 可独立编译与验证。依赖用 `[blocked-by]` 标注。

## Tasks

- [x] T1 `src/skills/catalog/types.rs`：`SkillCandidate` 增 `repo_id: Option<String>` / `repo_name: Option<String>`；新增 `RepoSource { id, name: Option<String>, path }`；`SkillsPaths` 增 `repos: Vec<RepoSource>`（保留 `root` 取首个 repo 向后兼容）。更新所有构造点使其编译（暂填 None/空）。单元测试覆盖 `RepoSource` 构造。
- [x] T2 `src/skills/config/mod.rs`：解析 `skills.repo[]`（name/path）为 `Vec<RepoSource>`；`skills.dir` 自动转单 repo；`dir`+`repo` 同存 → repo 优先 + deprecation warning；override（`--skills-dir`/`LLMAN_SKILLS_DIR`）全盘替代为单元素列表（D1）。`resolve_skills_root*` 返回 repo 列表，`ensure_dirs` 处理多 repo。单元测试：单 dir 兼容、repo 优先、override 全盘替代、name 缺失 fallback。`[blocked-by: T1]`
- [x] T3 `src/skills/catalog/scan.rs`：`discover_skills` 接受多 repo，扫描时写入 `repo_id`/`repo_name`；跨 repo 同名按列表顺序首个生效（D2），复用 `dedupe_skills` 冲突提示。单元测试：多 repo 携带正确 repo_id；跨 repo 同名去重 + 警告。`[blocked-by: T2]`
- [x] T4 `src/config_schema.rs`：`GlobalSkillsConfig` 增 `repo: Option<Vec<RepoEntrySchema>>`（name/path），`dir` 描述标 deprecated；更新 `Default for GlobalConfig`（默认仍单 dir）；重生成 `artifacts/schema/configs/en/llman-config.schema.json`；单元测试 schema 含 repo 定义且旧 dir 实例校验通过。`[blocked-by: T2]`
- [x] T5 新增 `llmanspec/specs/config-schemas/multi-repo-source-schema.feature` 的 `#[scenario]` 绑定与 given step（`全局 config.yaml 含 ...`）于 `tests/bdd_steps.rs`；复用 `运行 llman`/`退出码为`/`stderr 包含`。`cargo test --features bdd` r125 三场景全绿（full mode）。`[blocked-by: T4]`
- [x] T6 `src/skills/cli/command.rs`：`grouped_skill_options` 改两层分组（repo 顶层 + 目录名前缀次级，D3）；单 repo 退化不显 repo 头（D4）；多 repo 技能行旁显来源标签；`/` 搜索匹配 name/路径简称。单元测试：多 repo 分层、单 repo 退化、搜索匹配 repo 名。`[blocked-by: T3]`
- [x] T7 r126 的 5 场景由单元测试覆盖（fast mode，不绑 `#[scenario]`）：发现携带 repo 元数据、TUI 顶层 repo 分组、单 repo 不显头、跨 repo 同名首个生效 + 警告、搜索匹配。`[blocked-by: T6]`
- [x] T8 新增/更新 i18n key（deprecation warning、repo 分组标签等）仅 en；`just check` 全绿（fmt+clippy+test）；`llman sdd validate <id> --strict --no-check` 通过。`[blocked-by: T5,T7]`

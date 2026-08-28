# Tasks — src-cleanup-pre-split

Seam 确认（用户已批）：S1 = 既有回归网（tests/ 22 集成测试 + BDD
`cargo test --features bdd` + `llman --help` 快照），一切重构 task 的行为零变化
验收；S2 = 新 arch test（本 change 唯一新增测试）；S3 = 编译基线（design.md
记录，非断言）。无新 `.feature`（`skip_specs_landing: true`）。
决议引用：D-A…D-F 见 design.md。task 详情（验收口径/原则）见 design.md 各节。

- [x] T1: schema-ring-untangle — config_schema 拆两块（校验工具下沉 schema_utils，llmanspec schema 生成移入 sdd::project::config），环消失；调用方改引
- [x] T2: git-utils-merge — 新 src/git_utils.rs 收编 find_git_root + git_native 四函数 + init_repo/git_ref_exists 可归并实例；迁移全部调用点；skills/shared 清空则删
- [x] T3: small-utils-dedup — is_symlink_dir/normalize_newlines/push_unique/slugify 按「语义相同才合并、不同就改名」处置
- [x] T4: config-disambiguation — x::codex::Config→CodexConfig；ToolConfig/ToolsConfig 消歧；五处所有权与文件格式不动 [blocked-by: T1]
- [x] T5: arch-test-lock — tests/import_direction_tests.rs 断言方向表，挂进 just check [blocked-by: T1, T2, T3, T4]
- [ ] T6: sdd-shared-dedup — print_json ×3 合一；non_interactive_hint_message ×2 合一
- [ ] T7: sdd-cross-submodule-dedup — proposal_path/change_dir/list_specs/normalize_type 归位 sdd/shared；git 操作走 git_utils [blocked-by: T2]
- [ ] T8: sdd-visibility-scope — pub→pub(crate) 收敛（对外仅 command 入口 + schema 生成 API）；六子块边界检视追记 design [blocked-by: T6, T7]
- [ ] T9: x-shared-leaves — 新 src/x/shared/ 收编逐字相同叶子；不同构者注记留证；骨架参数化不做
- [ ] T10: compile-baseline — 冷全量/包级重建/热增量/du 盘占 + line-tables-only A/B，进 design 附表 [blocked-by: T5, T8, T9]
- [ ] T11: extract-llman-core — 顶层工具层抽 llman-core crate，门面重导出保路径零漂移 [blocked-by: T10]
- [ ] T12: extract-llman-sdd — src/sdd 整体搬 crates/llman-sdd（pub mod sdd 保内部路径），门面 pub use 重导出；test_utils 副本；i18n 接线 [blocked-by: T11]
- [x] T13: disk-hygiene-justfile — justfile 新增 clean-bdd-targets 配方
- [ ] T14: post-split-measure — 拆后同口径复测 + crate 边界图终稿 + just check-all 全绿 [blocked-by: T12]

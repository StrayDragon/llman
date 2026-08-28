# Tasks — src-cleanup-pre-split

Seam 确认（用户已批）：S1 = 既有回归网（tests/ 22 集成测试 + BDD
`cargo test --features bdd` + `llman --help` 快照），一切重构 task 的行为零变化
验收；S2 = 新 arch test（本 change 唯一新增测试，断言 `use crate::` 方向表）；
S3 = 编译基线（design.md 记录，非断言）。无新 `.feature`
（`skip_specs_landing: true`）。决议引用：D-A…D-F 见 design.md。
完成时按归档惯例逐条改写为行内 `- [x] Tn: … [blocked-by: …]`。

## T1 schema-ring-untangle
config_schema 拆两块：`validate_yaml_value`/`prepend_schema_header` 下沉新
`src/schema_utils.rs`；`generate_schema::<SddConfig>` 移入 `sdd::project::config`
并导出 llmanspec schema 生成函数，config_schema 的 Llmanspec 分支改调它（环消失）；
tool/config、skills/config、sdd/project/config 改引 schema_utils；评估
LLMANSPEC_DIR_NAME 常量归属。行为零变化（S1）。
依赖：无

## T2 git-utils-merge
新 `src/git_utils.rs` 收编 find_git_root + git_native 四函数 + init_repo/
git_ref_exists 可归并实例（逐函数审阅，sdd 专属语义留 sdd）；迁移 tool/sync_ignore、
tool/agents_md、sdd/change/{archive,finalize,lock_gate,start} 调用点；
skills/shared 清空则删目录。
依赖：无

## T3 small-utils-dedup
is_symlink_dir / normalize_newlines / push_unique / slugify 按「语义相同才合并、
不同就改名」处置（design T3 原则，先 diff 实现再动）；合并位 = 顶层工具层。
验收：grep 无同名同义散布。
依赖：无

## T4 config-disambiguation
`x::codex::Config → CodexConfig`；`ToolConfig`/`ToolsConfig` 查实际用途后合并
或改名消近名；五处 config 模块所有权不动、YAML/TOML 格式不变（S1 配置测试）。
依赖：T1

## T5 arch-test-lock
新 `tests/import_direction_tests.rs`：fs 扫描 `src/**/*.rs` 的 `use crate::`
断言 design 方向表（sdd/skills/tool/x/顶层工具层五行）；确认在 `just check`
的 cargo test 中生效。contract 收口：此后方向违规即红。
依赖：T1, T2, T3, T4

## T6 sdd-shared-dedup
sdd/shared 自身去重：`print_json` 同签名 ×3 合一；`non_interactive_hint_message`
×2 合一。
依赖：无

## T7 sdd-cross-submodule-dedup
sdd 子模块间重复收敛：proposal_path / change_dir / list_specs / normalize_type
归位 sdd/shared 既有文件；init_repo/git 操作统一走 git_utils。
依赖：T2

## T8 sdd-visibility-scope
sdd 内部 `pub → pub(crate)` 收敛（对外仅 command 入口面）；六子块
（change/project/shared/spec/context/authoring）边界检视，结论追记 design
（crate 边界图输入）。
依赖：T6, T7

## T9 x-shared-leaves
新 `src/x/shared/`：收编逐字相同叶子（mask_secret、is_empty、no_configs_message
等，逐个 diff 确认同构）；19 同名清单中不同构者保留并在 task 注记留证。
骨架参数化不做（D-A）。
依赖：无

## T10 compile-baseline
拆前基线（design 测量方案表）：冷全量 / 包级重建 / 热增量 / du 盘占 +
`line-tables-only` A/B，数据进 design.md 附表。
依赖：T5, T8, T9

## T11 extract-llman-core
顶层工具层（fs_utils/path_utils/managed_block/env_safety/git_utils/schema_utils）
抽 `llman-core` crate；门面 `pub use llman_core::…` 重导出保路径零漂移；
成员 crate i18n!/build.rs 接线（design crate 边界图）。
依赖：T10

## T12 extract-llman-sdd
`src/sdd` 整体搬 `crates/llman-sdd`（git mv）；llmanspec schema 生成随之；
门面 `pub use llman_sdd as sdd;`；test_utils 81 行 cfg(test) 副本进 sdd crate；
t! 全量回归（S1 BDD + --help 快照）。
依赖：T11

## T13 disk-hygiene-justfile
justfile 新增 `clean-bdd-targets` 配方清理 `target/bdd-*`（r48 治理 why 已记录
于 proposal）。
依赖：无

## T14 post-split-measure
拆后同口径复测（T10 表）+ crate 边界图终稿写进 design + `just check-all` 全绿；
若阶段 1/2 触发弹性降级（D-C），本 task 改为冻结蓝图并收尾。
依赖：T12

# Tasks — specs-flat-layout

Seam 确认（需求方已批，D1-D12 见 design.md §1）：S1 = 既有回归网（BDD
`cargo test --features bdd` 的 9 个既有 spec-format/migrate 场景 + `sdd_bdd_compat.rs`
+ `validate --all --strict`）保持绿（兼容门）；S2 = 本 change 新增 @executable
场景（design.md §7.1 九个种子）与 Rust 单元测试（§7.2）；S3 = r131 编号保留、行为改写，
r136 引用不改。

任务按垂直切片拆：每条独立可验收、可单独 review；依赖用 `[blocked-by]` 标注。
任务细节（验收口径/边界/报告格式）一律以 design.md 对应节为准，不在此复述。

- [x] T1: spec-loc-resolution — 引入 `SpecLoc { id, path }`（§3.3）：`list_spec_locs` 收扁平文件与目录两形态；`resolve_spec_file` 实现 id 推导/优先级/backfill/冲突 ERROR（§3.1-3.2）；`# capability:` 匹配改为对 id（§2.1 第三行）；所有命令输出/错误路径改走 SpecLoc.path，禁止再拼 `specs/<id>/` [blocked-by: 无]
- [x] T2: skeleton-flat-default — `authoring/spec.rs::run_skeleton` 默认写 `specs/<cap>.feature`（不建目录）；存在冲突检查（扁平文件存在即拒，除非 --force）；帮助文案同步（§8 command.rs 行） [blocked-by: T1]
- [x] T3: r131-rewrite — spec-format.feature r131 条文替换为设计稿版本（§4）+ 新增 migrate rule（r13x，§7.1 场景 5-9 的 @req 挂载）+ 9 个 @executable 场景文案落地 [blocked-by: T1, T2]
- [x] T4: migrate-specs-flatten — `command.rs` kind 解析扩为 `["toon2features","specs-flatten"]`（`spec-md2toon`/partitioned 仍拒绝）；migrate.rs 新增分支：处理范围筛选（§5.2）、6 类预检查报告（§5.3）、git mv + 删空目录、scope 自引用改写（§5.4）、--dry-run、幂等、本轮不落 repo 迁移 [blocked-by: T1]
- [x] T5: migrate-prompt — `--prompt` flag（打印 §6.1 模板并返回 0，不执行）；新增 `units/migrate-prompt.md`（zh-Hans + en，过 check-sdd-templates 门禁）；specs-flatten 执行输出追加「— scope 检查 —」块（§5.5 形态） [blocked-by: T4]
- [x] T6: docs-sync — 根 AGENTS.md 单轨段、templates units（validation-hints/feature-contract）、skills zh/en 中 `specs/<capability>/<capability>.feature` 字样统一为「扁平 `specs/<capability>.feature` 或目录形态」；header 匹配 Warning 文案 t! 键更新（§9-1）；`just check-sdd-templates`、`just readme` 全绿 [blocked-by: T3, T4, T5]
- [x] T7: tests-thicken — `sdd_bdd_compat.rs` 增补：skeleton 扁平产出断言、migrate kind 解析、`resolve_spec_file` 优先级矩阵单测（§3.1 表）；确认既有目录形态 fixture（sample/sample3/legacy）零回归 [blocked-by: T3, T4]
- [x] T8: full-gate — `just check-all`（fmt/lint/nextest/BDD/docs/sdd-templates）全绿；本 change 交付机制与文档，仓库本体跑 specs-flatten 另开 quick change（§9-5） [blocked-by: T5, T6, T7]

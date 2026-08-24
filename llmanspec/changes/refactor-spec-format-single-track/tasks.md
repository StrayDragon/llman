# Tasks

垂直切片；每个 task 可独立验证。seam 已确认：CLI 子进程（泛化 step）/ Git-native 门禁 fixture / 库内函数单测。

- [ ] t1 FeatureBackend 与解析层
  - 新增 `src/sdd/spec/backend/feature_backend.rs`：头注释（capability/purpose/scope）+ tag 场景（@req/@human/@manual/@executable）→ `MainSpecDoc` 填充；GWT 槽位可选、description 承载 statement。
  - 锁定哈希规范化函数（D4 规则）+ 中文场景名/步骤的单元测试。
  - 验证：`cargo +nightly test --lib feature_backend`

- [ ] t2 validate 门禁换血
  - 删 dual-write、Partitioned 权威、BDD-off 分叉检查；增 tag 语法学、@human 归一化查重、孤儿 acceptance WARNING、三态计数。
  - 遗留 spec.toon → ERROR 指向 toon2features。
  - 验证：`cargo +nightly test --lib validation` + 手工 smoke `llman sdd validate --all --strict --no-check`

- [ ] t3 子命令输出适配 `[blocked-by: t1]`
  - `list --specs`/`show` 三态 morphology + 覆盖矩阵；`index rebuild` 两类场景带 req_id 入树（r78 改写）；`context` 携带分级标记；`resolve-req`/`next-req-id` 改扫 feature 标签；`spec scaffold` 单文件骨架。
  - 验证：对应集成测试 `tests/*_tests.rs`

- [ ] t4 migrate --kind toon2features `[blocked-by: t1]`
  - 幂等转换（D5）；迁移报告含三态初值；`--kind spec-md2toon` 退役为 ERROR 提示合法 kind。
  - 验证：临时目录 fixture 集成测试（TempDir，禁止污染仓库根）

- [ ] t5 change 门禁挂点 `[blocked-by: t2]`
  - specsLanding glob 收窄 `*.feature`；finalize/checkpoint/diff 接锁定哈希对比（base_sha...HEAD）；frontmatter 合法字段集增 `rules_edit_acked`（r124 + schema 同步）。
  - 验证：Git-native fixture 测试（拦截路径 + acked 解锁路径两案例）

- [ ] t6 全库自迁移 `[blocked-by: t2, t4]`
  - 跑 toon2features 迁移 28 个 capability；人工审阅生成物中 @human 场景；config.yaml 保持 runner 开关语义；`bdd.bindings` 退役。
  - 验证：`llman sdd validate --all --strict` 全绿；`cargo test --features bdd` 通过

- [ ] t7 compat tests 与受影响 specs 重写 `[blocked-by: t2]`
  - `tests/sdd_bdd_compat_tests.rs` smoke/read_only 列表更新；sdd-bdd-mode-compat 相关 `.feature` 按 runner 开关收缩后的合约重写（可执行场景走泛化 step）。
  - 验证：`cargo +nightly test --test sdd_bdd_compat_tests`

- [ ] t8 模板、skills 与文档改版 `[blocked-by: t6]`
  - `templates/sdd/**`、`.agents/skills/llman-sdd-*`、根/llmanspec AGENTS.md 托管块同步单轨叙事；产出下游迁移 prompt 文档；术语债清理（harness → rule 三态口径）。
  - 验证：`just check-sdd-templates`

- [ ] t9 死码清除与收尾 `[blocked-by: t6, t7]`
  - 删 ToonBackend/toon-format 依赖/partitioned.rs 双写机器；`just check-all` 全绿（含 release build 与 rustdoc -D warnings）。
  - 验证：`just check-all`

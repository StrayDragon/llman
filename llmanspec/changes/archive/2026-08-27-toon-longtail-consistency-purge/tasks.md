# Tasks — toon-longtail-consistency-purge

Seam 约定（S1–S4，用户已确认）：均为既有 BDD harness 驱动的 CLI 子进程边界：
S1=`validate` 系列 / S2=`project migrate` / S3=`spec skeleton` / S4=`show`+`list --specs`。
新增验收一律为挂现有 `@req` 的纯 `@executable` 场景；错误消息遵循 D1 友好三要素。
Specs landing commit：65cda5d（+r114/r79 修订与新 executable 场景随后落地）。

- [x] T1 contracts-align-secondary-specs: templates 双载体教学段收窄（唯一过期点 zh arch-review L63）+ init --update 重渲染 .agents + check-sdd-templates 绿; root AGENTS.md 复核无需改 [blocked-by: none]
- [x] T2 md2toon-retire-verdict: clap value_parser 已只认 toon2features（stderr 含该词，契约 executable 过）; Migrate/Skeleton clap 文档重写单载体; dispatch 反注释修正; locales 死区块（solidify×4 + partition_migrate×6）删除 [blocked-by: T1]
- [x] T3 r60-deletion-and-show-dedual: show spec --json 移除 constraints/harness 双字段与 harness_summaries 构建块; morphology/r39 形态回归不变; S4 负断言由既有 compat+bdd 场景覆盖 [blocked-by: T1]
- [x] T4 skeleton-single-carrier-authoring: 实现核验已单载体（仅 .feature、next-req-id、--force 门、strict 直过）; clap long-help 嵌入骨架格式示例满足「help 嵌示例」条款; r114 增补修订（@executable 示例默认不生成，防下游绑定面扩大）随 landing 提交 [blocked-by: T1]
- [x] T5 context-single-carrier-retrieval: 实现核验 mod/index/tree 已 .feature-only（遗留 toon=跳过目录+警告指 migrate）; compute_spec_hash 忽略 toon 有专测; 旧 tree.json 无 scenarios 字段加载兼容保留; r79 措辞与实现对齐修订随 landing 提交 [blocked-by: T1]
- [x] T6 gate-polish-and-clean-sweep: resolve_spec_file 三要素报错（路径+原因+migrate 指引）; 新增 executable legacy-spec-toon-error-message-is-actionable; 附带 out-of-scope 门禁修复 nightly needless_bool(tree_sitter_processor); 自修复 R2=i18n 缓存致 live 关键词丢失回归(compat 测试抓到)。门禁全绿：clippy -D 0、非 BDD 全量 ok(414 lib+集成)、BDD runner 51/51、validate --all --strict 35/35、check-sdd-templates 通过 [blocked-by: T2, T3, T4, T5]

自修复轮次记录：
- Round 1：clippy 新 lint needless_bool 打在无关存量文件 → 精准套用 clippy 建议（独立 fix(tool) commit）→ clippy_exit=0
- Round 2：rust-i18n 宏缓存未感知 app.yml 变更，stderr 仍吐旧文案丢 "live" 关键词，被 test_validate_change_next_steps_branches_on_bdd_mode 捕获 → touch src/lib.rs 强制重编后恢复；文案同步补回 live 单载体措辞

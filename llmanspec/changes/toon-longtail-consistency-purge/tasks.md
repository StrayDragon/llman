# Tasks — toon-longtail-consistency-purge

Seam 约定（S1–S4，用户已确认）：均为既有 BDD harness 驱动的 CLI 子进程边界：
S1=`validate` 系列 / S2=`project migrate` / S3=`spec skeleton` / S4=`show`+`list --specs`。
新增验收一律为挂现有 `@req` 的纯 `@executable` 场景；错误消息遵循 D1 友好三要素
（定位到的文件路径 + 一句话原因 + migrate 指引），对齐 errors-exit/cli-experience 风格。
依赖序：T1 → ｛T2, T3, T4, T5｝ → T6（expand-contract：先统一口径，再分片拔旧，末片清场）。
Specs landing 已在绑定分支完成（commit 65cda5d），下列切片均指 src/templates/locales 实现侧。

## T1 contracts-align-secondary-specs
templates/sdd 中 propose/verify/archive/arch-review/validation-hints 的双载体教学段
同步为单载体口径（en/zh-Hans parity，过 just check-sdd-templates）；
只动教学文本不改行为。已核：root AGENTS.md 无需改动。
Blocked-by: 无

## T2 md2toon-retire-verdict
D2 落实实现核对与收口：`project migrate --kind spec-md2toon` 必须非零退出且 stderr
友好提示仅支持 toon2features（S2 seam；沿用/增强 executable
migrate-spec-md2toon-retired 的 stderr 三要素断言）；清理 locales/app.yml 中
md2toon 成功文案死串（app.yml L1867-1868 及相关 key）。
Blocked-by: T1

## T3 r60-deletion-and-show-dedual
拆除 shared/show.rs（及 list 关联路径）中 Constraints/Harness 双源分段渲染代码与
associated JSON 字段（合约侧 r60 已删）；morphology 口径回归不变（r39/r134）。
S4 seam 负断言：show spec-id 输出无分段字样；list --specs 三态计数形态不变。
Blocked-by: T1

## T4 skeleton-single-carrier-authoring
r114 重写落地：`spec skeleton` 仅生成 `<capability>.feature` 单载体骨架
（头注释 + 示例 @req/@human 场景 + 可选示例 @executable）；复用 next-req-id；
产物过 validate --strict；--force 外拒绝覆盖；help/error 内嵌格式示例。
src/authoring/spec 相应改造 + S3 seam 新 executable 场景。
Blocked-by: T1

## T5 context-single-carrier-retrieval
sdd-context r58/r79 重写落地：compute_spec_hash 仅哈希 `.feature`；
context/{index,retrieve,tree} 移除 toon 内容读路径；畸形/缺失 `.feature`
报错口径对齐 spec-format r131；**保留**旧 tree.json 缺 scenarios 字段的加载兼容。
相关单测翻新（retrieve.rs/tree.rs/index.rs 内嵌 tests）。
Blocked-by: T1

## T6 gate-polish-and-clean-sweep
S1 门禁体验收口：legacy-spec-toon-fails-with-pointer 断言升级为 stderr 三要素
（路径+原因+migrate 指引，无堆栈 token 友好）；validation/discovery 提示串统一走
locales key；全量清场：`llman sdd validate --all --strict`（含 full mode）、
`cargo test --features bdd`、`just check-sdd-templates`、
`cargo +nightly clippy --all-targets --all-features -- -D warnings`。
Blocked-by: T2, T3, T4, T5

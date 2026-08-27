# Tasks — toon-longtail-consistency-purge

Seam 约定（S1–S4，用户已确认）：均为既有 BDD harness 驱动的 CLI 子进程边界：
S1=`validate` 系列 / S2=`project migrate` / S3=`spec skeleton` / S4=`show`+`list --specs`。
新增验收一律为挂现有 `@req` 的纯 `@executable` 场景；错误消息遵循 D1 友好三要素
（定位到的文件路径 + 一句话原因 + migrate 指引），对齐 errors-exit/cli-experience 风格。

## T1 contracts-align-secondary-specs
规格文本收窄第一批 + 技能模板同步：bdd-mode-compat r26、structured-skill-prompts r65/r96
的 live `.feature` 措辞落单载体口径；templates/sdd 中 propose/verify/archive/
arch-review/validation-hints 的双载体教学段同步（en/zh-Hans parity）。
不改 src 行为（这些规则描述的行为已由 spec-format r131 实现）。
- [x] landing 落地（见 specs-laning commit）
- [ ] apply 时完成模板 parity

## T2 md2toon-retire-verdict
D2 落实：sdd-workflow r115 语句修正 + root AGENTS.md 常用命令行同步；
src 分派核对——`project migrate --kind spec-md2toon` 必须非零退出且 stderr 友好提示
（S2 seam，沿用 executable `migrate-spec-md2toon-retired` 断言增强 stderr 三要素）；
清理 locales/app.yml 死文案（md2toon 成功提示串）。
- [blocked-by: T1]

## T3 r60-deletion-and-show-dedual
删除 sdd-workflow r60 整段；拆除 shared/show.rs（及 list 相关）中
Constraints/Harness 双源分段渲染路径与 associated JSON 字段；morphology 口径回归不变。
S4 seam 负断言：show spec-id 无分段字样输出，list --specs 三态计数形态不变（r134）。
- [blocked-by: T1]

## T4 skeleton-single-carrier-authoring
sdd-workflow r114 重写落地：`spec skeleton`（命令名对齐 r133）仅生成
`<capability>.feature` 骨架（# language/# capability/# purpose/# scope 头 +
示例 @req/@human 场景 + 可选示例 @executable）；复用 next-req-id；产物过
`validate --strict`；`--force` 外拒绝覆盖；help/error 内嵌格式示例。
src/authoring/spec 相应改造。S3 seam 新 executable 场景覆盖单载体骨架断言。
- [blocked-by: T1]

## T5 context-single-carrier-retrieval
sdd-context r58/r79 重写落地：compute_spec_hash 仅哈希 `.feature`；index/retrieve/
tree 移除 toon 内容读路径；畸形/缺失 `.feature` 按 r131 报错口径；
**保留**旧 tree.json 缺 `scenarios` 字段的加载兼容（bdd-mode-compat 明文）。
相关单测翻新。
- [blocked-by: T1]

## T6 gate-polish-and-clean-sweep
S1 门禁体验收口：legacy-spec-toon-fails-with-pointer 断言升级为 stderr 三要素
（路径+原因+migrate 指引，无堆栈 token 友好）；validation/discovery 提示串统一走
locales key；全量清场：`llman sdd validate --all --strict`（含 full mode）、
`cargo test --features bdd`、`just check-sdd-templates`、`cargo +nightly clippy -D warnings`。
- [blocked-by: T2] [blocked-by: T3] [blocked-by: T4] [blocked-by: T5]

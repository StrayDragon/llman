# Tasks: fix-toon2features-preserve-features

## 测试边界（seam）

- seam：CLI 子进程 `llman sdd project migrate --kind toon2features --yes`（泛化 step `在非交互终端运行 llman {args}` 直驱）
- 断言面：`相对路径 {rel} 存在/不存在/内容包含 {text}`（既有）+ `stdout 包含 {text}`（记账行）

## 垂直切片

### t1: Specs landing——r136 合约改写 + BDD 场景
- [ ] `spec-format.feature` r136 @human 描述改写（只处理 spec.toon；既有 .feature 不动；GWT 行→@human；语言链；skip；记账）
- [ ] executable 场景改写：`migrate-toon2features-converts-and-cleans`（仅 toon fixture）；`migrate-toon2features-keeps-features-and-converts-gwt-notes`（不动 .feature + @human 转写 + en 渲染）；新增 `migrate-toon2features-skips-when-main-feature-exists`

### t2: backend 语言感知渲染
- [ ] `feature_backend.rs` 增 `dump_main_spec_lang(doc, lang)`（keywords_for 驱动 头/功能/场景/步骤）；`dump_main_spec` 委托 zh-CN
- [ ] 单测：zh-CN/en 快照断言

### t3: migrate 核心重写
- [ ] 语言检测链（bdd.default_language > locale > .feature 头嗅探 > en）+ 单测
- [ ] toon scenarios 分流：GWT+配对 → @human；未配对 → dropped_unpaired；无 GWT → dropped_notes；feature 列不分支 + 单测
- [ ] 既有 .feature 不读不写不删 + `left N` 记账；主 .feature 存在 → skip（保留 toon）+ 单测
- [ ] 关键字前缀剥离（en/zh 双向）+ 单测；dry-run 对齐新记账；幂等
- [ ] 删除死码 `extract_scenario_blocks` 及其测试；`tests/bdd_steps.rs` fixtures 新增/复用

### t4: 验收 + 复现
- [ ] `cargo +nightly test`（全量）+ `cargo +nightly test --features bdd` 全绿
- [ ] `llman sdd validate --all --strict` 全绿
- [ ] worktree `wt/toon2features-replay` 重置 a275d6a 用新二进制复现：旧 .feature 全部保留、toon→@human、spec-format skip

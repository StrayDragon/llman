---
depends_on: []
---

工具链 quick 路径候选（源自 toon-longtail-consistency-purge apply R2 自修复实录，
commit 4080890）。姊妹草案：fix-tasks-checkbox-parse-trap（同为开发者体验类小修）。

## Why

`i18n!("locales")` 过程宏在编译期嵌入翻译串，但不追踪 locale 文件变更：
修改 `locales/app.yml` 后直接 `cargo test`，测试二进制仍携带旧文案——
apply 阶段曾因此出现「next-step 提示丢失 live 关键词」的假回归，
被迫 `touch src/lib.rs` 强制重编才暴露真实结果。静默陈旧对 CI 与本地同等危险。

## What Changes

- 增加 build.rs（或等价机制）对 `locales/**` 声明
  `cargo:rerun-if-changed`，使 yml 变更可靠触发重编。
- 复核 rust-i18n 版本是否已有内建追踪开关；有则升级启用替代自写 build.rs。
- 在 AGENTS.md 测试节补一行备忘：改翻译后无需手工 touch 即生效。

## Non-goals

- 不迁移到运行时加载方案（保持编译期内嵌、二进制自包含）。
- 不引入翻译格式/schema 变化。

## Verification Sketch

- 手工配方进 tasks：改一个 value → `cargo test -q --test sdd_bdd_compat_tests`
  无需 touch 直接断言新文案出现。
- `touch locales/app.yml && cargo build` 增量时间开销记录在案（预期毫秒级）。

## Open Questions

- rust-i18n 4.x upstream 是否已修复（若已修，本草案降级为仅补 AGENTS.md 备忘）。

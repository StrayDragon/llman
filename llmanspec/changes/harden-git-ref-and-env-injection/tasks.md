# Tasks: harden-git-ref-and-env-injection

## 1. Specs / proposal

- [x] 创建 proposal / design / delta specs（sdd-workflow、codex-account-management、claude-code-runner）
- [x] `llman sdd validate harden-git-ref-and-env-injection --no-interactive`（deltas/structure OK；`--strict` 会因 pending tasks 报 ERROR，属预期）

## 2. Git base-ref hardening

- [x] 在 `src/sdd/spec/staleness.rs`（及相关 git 调用）校验 `LLMANSPEC_BASE_REF`
- [x] 拒绝空 / 以 `-` 开头；argv 加 `--` 隔离
- [x] 单测：option-like ref 失败且不调用危险 git option

## 3. Env injection denylist

- [x] 共享危险键检测 helper（`LD_PRELOAD` / `LD_LIBRARY_PATH` / `PATH` / `DYLD_*`，大小写不敏感）
- [x] Codex 子进程注入路径：命中 denylist → 失败且不启动
- [x] Claude Code 子进程注入路径：同上；复用/对齐 `env_injection` 语法校验
- [x] 单测：危险键拒绝；安全键仍可注入

## 4. Verification

- [x] `cargo +nightly test`（相关模块）
- [x] `cargo +nightly clippy --all-targets --all-features -- -D warnings`
- [x] `llman sdd validate harden-git-ref-and-env-injection --strict --no-interactive`

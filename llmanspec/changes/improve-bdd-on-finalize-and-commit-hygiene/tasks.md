# Tasks: BDD-on 单 commit 收尾（`finalize`）

> 顺序执行。每步完成后勾选。校验命令见各段。

## 1. 合约层（spec.toon + .feature）

- [ ] 1.1 在 `llmanspec/specs/sdd-bdd-mode-compat/spec.toon` 的 `requirements[]` 表新增 r94
      （title：`BDD-on finalize 单 commit 收尾`；statement 含 MUST/SHALL 见 proposal §1）。
- [ ] 1.2 在同 spec.toon 的 `scenarios[]` 表新增一行 r94 不可执行 scenario
      （`id: finalize-single-commit-note`，`feature: false`，指向 .feature）。
- [ ] 1.3 在 `llmanspec/specs/sdd-bdd-mode-compat/git-binding.feature` 末尾新增 `@executable @req:r94`
      可执行场景（至少覆盖：成功路径 + gate 失败路径），优先复用泛化 step
      （`当 运行 llman {args}` / `那么 退出码为 {code:i32}` / `那么 stderr 包含 {text}`）。
- [ ] 1.4 运行 `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run --quiet -- sdd validate sdd-bdd-mode-compat --strict --no-check` 通过。

## 2. 实现层（Rust）

- [ ] 2.1 `src/sdd/command.rs`：在 `SddChangeCommands` enum 新增 `Finalize { change, no_check, no_interactive }`，
      并在 `SddCommands::Change` 的 match 中 dispatch（参考 `Checkpoint` 的 470-482 / 714-724）。
- [ ] 2.2 `src/sdd/change/git_native.rs`：新增 `FinalizeArgs` 与 `run_finalize(root, args)`：
      - 读 binding；幂等检查（若 `checkpointed && checkpoint_sha.is_some()` → 跳过写入）；
      - gate（attach / branch match / 非默认分支 / feature_delta 拒绝）—— **不含 clean tree**；
      - `validate` 两次（live strict + change stage），`--no-check` 跳过 runner；
      - 写 binding：`checkpointed = true`，`checkpoint_sha = binding.base_sha.clone()`；
      - 调用抽取出的 BDD-on archive rename 段（见 2.3）。
- [ ] 2.3 **抽取共享函数**（design D5 路线 1）：
      - 把 `enforce_bdd_archive_gates` 的 clean-tree 检查拆出，得到
        `enforce_bdd_archive_gates_relaxed`（finalize 用）；
      - 把 archive rename 段（`archive.rs:133-164`）抽成 `do_bdd_on_archive_rename(root, change_id)`，
        供 `archive::run_with_root` 与 `run_finalize` 复用。
- [ ] 2.4 帮助文本：`change finalize --help` 明确说明
      「`checkpoint_sha` 写入的是 `base_sha`（attach 时的 merge-base），不是实现 commit 的 HEAD；
      若需后者请用 `change checkpoint` + `change archive`」。
- [ ] 2.5 单元测试：`run_finalize` 的幂等性、gate 失败不改文件、sha 写入 = base_sha。
- [ ] 2.6 `just fmt` + `just lint` 通过（clippy `-D warnings`）。

## 3. 兼容性测试同步

- [ ] 3.1 `tests/sdd_bdd_compat_tests.rs::test_all_subcommands_smoke_bdd_on_and_off`：
      在 `read_only: false` 列表新增 `Finalize`。
- [ ] 3.2 如 1.3 的可执行场景需要新 step 模式，在 `tests/bdd_steps.rs` 底部加 `#[scenario]` 绑定
      （`name` 字节级匹配 .feature 场景标题；优先复用现有泛化 step）。
- [ ] 3.3 `just test` 通过。

## 4. Skills / 文档同步

- [ ] 4.1 `.agents/skills/llman-sdd-archive/SKILL.md`：新增推荐路径「单 commit 收尾：
      `llman sdd change finalize <id>`」，保留旧 5 步时序作 fallback。说明 sha 语义差异。
- [ ] 4.2 `.agents/skills/llman-sdd-apply/SKILL.md` + `llman-sdd-verify/SKILL.md`：
      加「不要单独 commit 纯 draft」注记。
- [ ] 4.3 `docs/release/partitioned-ssot/UPGRADE_AGENT_PROMPT.md`：补 finalize 章节
      （attach → 实现 → `finalize <id>` → 单 commit）。
- [ ] 4.4 `just check-sdd-templates` 通过。

## 5. 最终门禁

- [ ] 5.1 `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run --quiet -- sdd validate --all --strict --no-check` 全绿。
- [ ] 5.2 `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run --quiet -- sdd validate --all --strict --check`（full mode）全绿。
- [ ] 5.3 `just check` 通过（fmt + lint + tests）。
- [ ] 5.4 手动验收：临时 fixture 上跑 finalize 成功 + 单 commit 后 `sdd list` 无 active change。

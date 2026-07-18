# Tasks — improve-partitioned-ssot-agent-friction

## 代码组（合约层）

- [x] `src/sdd/spec/partitioned.rs::validate_partitioned`：dual-write message 列出具体的
      `(req_id, scenario.id)` 对。需要新加一个 helper（例如 `dual_write_pairs(doc, harness)`）
      返回 `Vec<(req_id, scenario_id)>`，在 message 里格式化为 `[(r12, login-ok), ...]`。
- [x] `src/sdd/command.rs::SddChangeCommands::Checkpoint`：加 `#[arg(long)] no_interactive: bool`
      字段，dispatch 时忽略（checkpoint 本身无 interactive 逻辑）。
- [x] 更新或新增单元测试覆盖上述两点（partitioned.rs 的 dual_write_pairs + command.rs flag 解析）。
      实现：新增 3 个单测（dual_write_pairs_lists_conflicts / _empty_when_no_conflict /
      validate_partitioned_dual_write_message_lists_pairs），全通过。
- [x] 同步合约层 BDD 场景：
      - `llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature`：双写场景的 Then
        断言加 `stderr 包含 (r` 之类（验证具体 id 出现在输出）。
        实现：新增场景「双写错误消息列出具体冲突对」@req:r6，断言 stderr 含 `(r1, happy)`。
      - `llmanspec/specs/sdd-bdd-mode-compat/git-binding.feature`：加新场景「checkpoint
        接受 --no-interactive 不报错」。实现：新增 @req:r57 场景，断言 stderr 不含
        `unexpected argument`。
      - `tests/sdd_bdd_compat_tests.rs::test_all_subcommands_smoke_bdd_on_and_off`：确认
        checkpoint flag 矩阵覆盖（如 smoke 列表需补 `--no-interactive`）。
        结果：smoke 测试无需改动即通过（flag 是纯加法）。
- [x] `tests/bdd_steps.rs`：若新场景的 step 不在泛化库中，新增最小 step 定义。
      结果：复用现有 `stderr 不含 {text}` / `stderr 包含 {text}` step，无需新增。

## 文档组（skills，非合约）

- [x] `.agents/skills/llman-sdd-propose/SKILL.md`：加 Partitioned SSOT 双写形状 2 列对照表
      （Executable scenario vs Doc-only scenario → toon / .feature）。
      实现：改 `templates/sdd/{en,zh-Hans}/skills/llman-sdd-propose.md` + `sdd init --update`。
- [x] `.agents/skills/llman-sdd-archive/SKILL.md`：加 3 阶段 commit 时序
      （commit live specs+code → checkpoint → commit checkpoint metadata → archive →
      commit archive rename）+ archived depends_on INFO 说明。
      实现：改 `templates/sdd/{en,zh-Hans}/skills/llman-sdd-archive.md` + `sdd init --update`。
- [x] `.agents/skills/llman-sdd-explore/SKILL.md` 和 `.agents/skills/llman-sdd-verify/SKILL.md`：
      加「诊断结构门禁优先 `validate <cap> --strict --no-check` 再 full」指引。
      实现：改 `templates/sdd/{en,zh-Hans}/skills/llman-sdd-{explore,verify}.md` + 重新生成。
- [x] 若 skills 有英文镜像文件，同步更新（按现有目录结构判断）。
      实现：templates 下 en + zh-Hans 两个 locale 均更新，`just check-sdd-templates` 通过。

## 校验组

- [x] `just fmt && just lint` 通过。
- [x] `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- sdd validate
      improve-partitioned-ssot-agent-friction --strict --no-interactive --no-check` 通过。
- [x] `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo run -- sdd validate --all
      --strict --no-interactive --no-check` 全绿（Totals: 31 passed, 0 failed）。
- [x] `cargo test --features bdd` 中新增/修改的 BDD 场景通过（full mode）。
      关键新场景：`sdd_bdd_mode_compat__gwt_validate_strict`（双写列 id）、
      `git_binding_change_checkpoint_no_interactive_flag` 均 ok；smoke 测试 ok。
- [x] `just check-sdd-templates` 通过（templates locale parity: en, zh-Hans）。

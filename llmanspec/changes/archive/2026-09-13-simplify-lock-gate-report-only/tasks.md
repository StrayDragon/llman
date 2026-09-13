# Tasks: simplify-lock-gate-report-only

垂直切片：每个 task 打穿 实现 → 测试 → 门禁 一条窄路径。Seam 确认：CLI 子进程（`llman sdd validate/finalize/diff`，复用既有 BDD step fixture）+ `lock_gate` 模块公共函数（单测）。

- [x] task-1: lock_gate 报告制 + 删除确认机器
  - `lock_gate.rs`：`check()` 返回 WARNING；删除 `LockedAck`/`locked_ack_for`/`agent_acked_for`/`undeclared_ids`/`ack_all_undeclared`/`ack_agent_marked`/`agent_marked_ids`/`upsert_frontmatter_id_list` 与 `LockedContent.agent_ids`；报告装配保留计数与 req-less 提示。
  - `finalize.rs`：删除 `confirm_locked_rules` 及调用；grep `--yes` 剩余用途，死标志从 CLI 移除。
  - `validate.rs`：删除 `yes_locked_ack` 分支与 `--yes` 标志（如无他用）；`show.rs` gateChecks 移除 lock-gate 项；`review.rs` 简化为锁定规则改动计数行。
  - spec backend/validation：`@agent` tag 配对校验删除；`PROPOSAL_FRONTMATTER_ALLOWED_FIELDS` 收缩（r124）。
  - 单测改写：ERROR 断言 → WARNING 断言；删除 ack 相关测试；保留 hash→id 反查与 req-less 提示（措辞去 rules_touched 化）；upsert/quote 测试随删除移除；消息计数断言按 D1 新口径。
  - 本 change proposal frontmatter 移除 `rules_touched`（新字段集生效的自举）。

- [x] task-2: BDD 场景与 step 适配
  - 删除场景：`yes-acks-agent-marked-rule-edit`、`yes-rejects-plain-rule-edit`、`yes-acks-deleted-agent-rule`、`agent-tag-without-human-rejected`。
  - 改写 `dup-req-removal-reports-req-id`：validate 退出码为零 + stderr 包含 `@req:r1`、WARNING、计数。
  - 新增场景：报告制不阻断——绑定 change 编辑锁定规则后 `validate --strict` 退出码为零且 stderr 含报告（复用 `变更 {change} 绑定且编辑了规则 普通` step，`{kind}=@agent` 分支删除）。
  - `tests/it/sdd_bdd_compat.rs` 中引用 rules_touched/agent_acked/@agent 的断言同步。

- [x] task-3: 文档 / 模板 / locale / schema 同步 [blocked-by: task-1]
  - r132/r135/r124/r130 文本已在 Specs landing；此处同步根 AGENTS.md 手写行、`crates/llman-sdd/templates/sdd/**`、locales（双拷贝）、`just check-schemas`、`just check-sdd-templates`、`just check-i18n-keys`、CLI 面变化时 `just readme`。

- [x] task-4: migrations/v0.0.77-v0.0.78/ [blocked-by: task-1]
  - 查 r28 既有 migrations 目录惯例；README prompt + 一次性脚本剥离 active change proposal 的 `rules_touched` / `agent_acked`（archived 免检不触碰）。

- [x] task-5: 全绿门禁 + E2E [blocked-by: task-2, task-3, task-4]
  - `just check-all`；`validate --all --strict`；`cargo test --features bdd`。
  - E2E 回归（复用 /tmp 最小复现仓）：绑定 change 删除 @agent 历史场景后 `validate`/`finalize` 全程零确认、零确认元数据、WARNING 报告可见、退出码为零。

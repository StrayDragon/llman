# Tasks: fix-lock-gate-ack-consistency

垂直切片：每个 task 打穿 spec 场景 → step fixture → 实现 → 测试一条窄路径，可独立验证。

- [x] task-1: hash→id 反查修复（重复 @req 场景删除按 req-id 报告且可豁免）
  - 实现：`lock_gate.rs` 的 `hashes_from_content` 返回值重构为 `{hashes, hash_to_id, agent_ids}`，`diff_edits` 用 hash→id 直接索引 req-id；`RuleEdit` 生成逻辑不变。
  - 单测：base 含两个 `@req:r1` 不同内容场景、双删 → 两条编辑均报 `@req:r1` 且 `rules_touched: [r1]` 后 gate 无 ERROR；同哈希多 id 取首个 id。
  - spec 场景：`spec-format.feature` 新增 `@executable dup-req-removal-reports-req-id`（给定 `变更 dup-fix 绑定且删除了重复 @req 锁定场景之一`，validate --strict --no-check 退出码非零、stderr 包含 `@req:r1` 与 `undeclared`）。
  - step fixture：`tests/bdd_steps.rs` 新增 given `变更 {change} 绑定且删除了重复 @req 锁定场景之一`（绑定前追加第二个 r1 场景并 commit，绑定后删除其一）。

- [x] task-2: `--yes` 标记解析含 base（被删 @agent 规则可自动确认）[blocked-by: task-1]
  - 实现：`agent_marked_ids` 合并 base 侧解析（`changed_feature_files` × `git show base:rel` 内容解析 locked+@agent 的 req-id，与工作树取并）；`hashes_from_content` 三件套结构被 `validate.rs --yes` 与 `finalize confirm_locked_rules` 两条路径共用。
  - 单测：删除 base 侧 `@agent` 规则后 `agent_marked_ids` 返回该 id；`ack_agent_marked` 将其写入 `rules_touched` + `agent_acked`。
  - spec 场景：`spec-format.feature` 新增 `@executable yes-acks-deleted-agent-rule`（给定 `变更 yes-del 绑定且删除了规则 @agent`，validate --strict --no-check --yes 退出码为零）。
  - step fixture：`tests/bdd_steps.rs` 新增 given `变更 {change} 绑定且删除了规则 {kind}`（绑定前落一个可选 `@agent` 标记的新锁定场景并 commit，绑定后删除）。

- [x] task-3: frontmatter 写入 quote_all [blocked-by: task-1]
  - 实现：`upsert_frontmatter_id_list` 改用 `to_string_with_options` + `quote_all: true`。
  - 单测：upsert 写入含 `20720420e754` 的列表 → 写出文本该条目带引号 → `locked_ack_for` / `agent_acked_for` 往返解析成功且值不变。

- [x] task-4: gate 错误行计数 [blocked-by: task-1]
  - 实现：`lock_gate_message` ERROR 文案追加 `{V} undeclared, {E} exempted`。
  - 断言：task-1 的 `dup-req-removal-reports-req-id` 场景 stderr 断言覆盖（包含 `1 undeclared`）。

- [x] task-5: 全绿门禁 [blocked-by: task-2, task-3, task-4]
  - `cargo test --features bdd`（BDD 套件转绿）+ `just check`（fmt/clippy/单测）。
  - `target/release/llman sdd validate fix-lock-gate-ack-consistency --strict` 通过；`llman sdd show fix-lock-gate-ack-consistency --json` 的 `readyToImplement=true`。
  - 用最小复现仓库（issue #18 场景：重复 @req 删除 + `--yes`）手工回归：`validate --yes` 与 `finalize --yes` 均能自洽通过。

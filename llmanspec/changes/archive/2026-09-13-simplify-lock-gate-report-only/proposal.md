---
depends_on: []
branch: sdd/simplify-lock-gate-report-only
base_sha: 5a02450d401a145e8715f53afbaccaf0250a453d
---

# 锁定规则门禁改为报告制（S0）

## Why

r135 锁定门禁的「ERROR 阻断 + `rules_touched` 输入侧确认列表」在真实使用中被证明是纯 token 浪费：

1. **scalim c60-specs-compaction 实测**：压缩 108 个 .feature（-6939 行），却花掉 ≥4 个 commit 与门禁搏斗（反复试错 req-id 列表、哈希列表、混合列表，最终 2026 token）；issue #18 的 2123 token 死锁即源于此。
2. **机制无强制力**：proposal frontmatter 与规则改动出自同一支笔（agent），精确声明列表拦不住恶意、只能抓粗心，而粗心在门禁报告里本来就会全部列出。声明↔diff 的匹配机器（hash→id 反查、base/工作树标记解析、引号处理）是 issue #18 三个 bug 的结构性根源。
3. **委托机制零使用**：`@agent`/`--yes`/`agent_acked` 在两个已知 dogfood 仓库（llman 自身 211 条、scalim 827 条规则）中标记数为 0。
4. 安全兜底本就存在：git 分支可以对比/回滚任何规则改动，`review`/`change diff` 是既定的人工控制点。S0 调研结论（S0/S1/S2 对比已定案）：报告制 + 既有 review 流程即可，确认元数据整体移除。

## What Changes

- lock-gate 降级为**报告制**：检出锁定 `@human` 场景的增删改，以 **WARNING** 报告（req-id 化 + 计数 + req-less 补标提示全部保留），MUST NOT 阻断 validate / change finalize / change diff。
- 删除确认元数据与机制：`rules_touched`、`agent_acked` frontmatter 字段（r124 合法字段集收缩）、`@agent` tag（r132 保留字汇收缩）、`--yes` 锁定确认语义（r135）。
- 删除代码：`LockedAck` / `locked_ack_for` / `agent_acked_for` / `undeclared_ids` / `ack_all_undeclared` / `ack_agent_marked` / `agent_marked_ids` / `upsert_frontmatter_id_list` / finalize `confirm_locked_rules`；保留并复用 issue #18 修复的 hash→req-id 反查报告（`diff_edits` / `LockedContent`）。
- `show` gateChecks 移除 lock-gate 项（不再有可失败的门禁）；`review` 输出简化为锁定规则改动计数浮现。
- BDD 场景增删：删除 4 个 yes/agent 场景；`dup-req-removal-reports-req-id` 改为报告制断言（退出码为零 + WARNING + req-id + 计数）。
- 迁移：`migrations/v0.0.77-v0.0.78/`（README + 一次性脚本，剥离 active change proposal 中的 `rules_touched` / `agent_acked`；archived 免检，r118）。
- 同步：AGENTS.md 手写行、llmanspec/AGENTS.md 与模板（`just check-sdd-templates`）、locales 死键（`just check-i18n-keys`）、JSON Schema（`just check-schemas`）、README 托管段（CLI 面变化时 `just readme`）。

## Capabilities

- spec-format（r132 tag 字汇、r135 锁定哈希门禁）
- sdd-workflow（r124 frontmatter 合法字段集、r130 specs landing 锁定门禁引用）

## Impact

- `crates/llman-sdd/src/sdd/change/lock_gate.rs`（大幅瘦身，报告制）
- `crates/llman-sdd/src/sdd/change/finalize.rs`、`crates/llman-sdd/src/sdd/commands/{validate,show}.rs`、`crates/llman-sdd/src/sdd/review.rs`
- `crates/llman-sdd/src/sdd/spec/{validation.rs,backend}`（@agent tag 校验移除）
- `tests/bdd_steps.rs`、`tests/it/sdd_bdd_compat.rs`（如引用锁定门禁字段）
- 模板 / locales / schema / README / migrations
- 无数据迁移风险：archived proposal 免检；活跃 change 依赖 r28 一次性脚本。

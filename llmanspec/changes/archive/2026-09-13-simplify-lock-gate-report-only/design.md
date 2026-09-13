# Design: simplify-lock-gate-report-only

## D1 报告等级与浮现面

`lock_gate::check` 的返回从 ERROR 降为 **WARNING**。新消息口径（计数语义随确认机制消亡而简化）：

```text
locked @human scenarios were modified (spec-format r135 report-only; compare via
git branch diff or `llman sdd review`; {N} edited): {detail}
```

req-less（无 @req）编辑追加：`{M} edited rule(s) carry no @req tag — restore them or add `@req:<id>` tags`。浮现面（注意：全局 `apply_strict` 会把 WARNING 升级为 ERROR，lock-gate 条目在该函数中按 path 豁免——r135 报告制是 spec 级不变量，--strict 下也必须保持 WARNING）：

- `validate`（单 change / --all / --specs，strict 与否同语义）：WARNING 行，退出码不受影响。
- `change diff` / `show`：lock-gate issue 原样透传（新等级）；`show` gateChecks **移除 lock-gate 项**（无可失败门禁，JSON 形状变化按无兼容政策）。
- `change finalize`：删除 `confirm_locked_rules` 整段（含交互 y/n 与 `--yes` 确认）；锁定规则改动经由 finalize 内部 validate 的 WARNING 自然浮现。
- `review`：`locked` / `agent_acked` 段简化为一行锁定规则改动计数（保留「改动可被 review 看见」的价值）。

## D2 删除面（代码）

lock_gate.rs 删除：`LockedAck`、`locked_ack_for`、`agent_acked_for`、`undeclared_ids`、`ack_all_undeclared`、`ack_agent_marked`、`agent_marked_ids`、`upsert_frontmatter_id_list`、`consider`。保留：`diff_edits`、`LockedContent{hashes, hash_ids}`（`agent_ids` 字段随 @agent 删除）、`hashes_at` / `worktree_hashes` / `hashes_from_content`、`changed_feature_files`、`effective_range_base`（r130 范围语义不变）、报告消息装配。issue #18 修复的 hash→req-id 反查完整存活（报告仍需要它）。

连带清理：`validate.rs` 的 `yes_locked_ack` 分支与 `--yes` 标志（若 grep 证实无其他用途则从 CLI 移除并 `just readme`）；`finalize.rs` 的 `yes` 参数同理（先 grep 其他交互用途）；spec backend/validation 中 `@agent` tag 的配对校验（r132「单独出现 MUST 报 ERROR」）删除。

## D3 @agent tag 整体删除（决策已定）

调研数据（两仓库 0 使用）支持删除：r132 保留字汇收缩（@agent 从列表移除），四个 yes/agent `@executable` 场景与 `agent-tag-without-human-rejected` 删除，BDD fixture 的 `{kind}=@agent` 分支删除（步骤保留给报告制场景复用）。`@human` / `@manual` / `@executable` 字汇不变。

## D4 r135 / r132 / r124 / r130 文本（Specs landing 落地，此处仅记录口径）

- r135：保留哈希规范化与范围语义；「MUST 报 ERROR …除非 rules_touched 覆盖」改为「MUST 以 WARNING 报告 …MUST NOT 阻断 validate / finalize / diff（报告制）」；确认路径、--yes、agent_acked 审计句删除；hash→req-id 反查口径与 req-less 提示保留。
- r132：@agent 从保留字汇移除（含配对规则句）。
- r124：合法字段集收缩为 depends_on、blocks、branch、base_sha、needs_specs_change。
- r130：删除「未带 rules_touched 的 proposal MUST NOT…」与「--yes 仅对 @agent…」句，保留范围对比 MUST（报告语义指向 r135）。

## D5 迁移与文档同步

- `migrations/v0.0.77-v0.0.78/`：README prompt + 一次性脚本（剥离 active change proposal frontmatter 的 rules_touched / agent_acked；archived 免检）。按 r28 惯例放置（先查既有 migrations 目录形态）。
- 模板：`crates/llman-sdd/templates/sdd/**` 中 frontmatter SSOT 段、AGENTS 渲染块、proposal 模板描述同步；`just check-sdd-templates`。
- locales：删除/更新引用 rules_touched、agent_acked、@agent 的键；`just check-i18n-keys`（双拷贝同步）。
- schema：proposal frontmatter JSON Schema 重生成（`just check-schemas`）。
- 根 AGENTS.md 手写行（「Locked rules (@human)」表行、SDD 增强 @agent 行）与 BDD 兼容维护注记同步。
- 自举注意：apply 落地新代码后，本 change 自己 proposal 里的 `rules_touched: [r124, r130, r132, r135]` 会因 r124 字段收缩被判 unknown field——task-1 中随代码删除一并移除（archived 后同样免检）。

## 风险

- 行为面收缩较大（CLI 标志、show JSON、报告等级），下游脚本若依赖 lock-gate ERROR 退出码会受影响——按 q9 无兼容政策处理，README 托管段更新覆盖。
- 旧二进制 + 新 specs 的窗口期：本 change 的 propose/apply 早期仍由旧门禁治理，靠 proposal 的 `rules_touched: [r124, r130, r132, r135]` 过渡，task-1 落地后即移除。

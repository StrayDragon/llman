---
depends_on: []
skip_specs_landing: true
---

# apply 阶段 Commit 纪律指引

## Why

`templates/sdd/*/skills/llman-sdd-apply.md` 全文没有任何 commit 指引（grep
"commit" 零命中），agent 在 apply 期按各自通用习惯自由提交。实证：
`src-cleanup-pre-split` 一个 change 在 main 留下 14+ 个 commit——逐 task
提交（T1–T14）、重复的 branch binding commit、独立的 skills resync commit，
四类内容混在步骤式历史里，reviewer 只能靠读裸 diff 拼语义。
需在 apply skill 模板中显式声明 commit 纪律：实现期不逐 task commit，
checkbox 勾选只改工作区，语义 commit 收敛到收尾一次性提交。

## What Changes

- `templates/sdd/{en,zh-Hans}/skills/llman-sdd-apply.md`：新增「Commit
  策略」节，内容要点：
  - apply 循环内（含自修复轮次）MUST NOT 逐 task commit；改动保持在工作区。
  - tasks.md checkbox 勾选只改工作区文件，不单独成 commit。
  - 默认收尾：verify 全绿后由 `change finalize` 单 commit 收尾（实现 +
    frontmatter + archive 改名一次提交）；仅在用户要求中途快照或需严格
    `checkpoint_sha` 时走 archive skill 的多 commit fallback。
  - 遇 blocker 停止时，把已完成改动一次性 commit 为 WIP 快照再报告。
- `llman sdd init --update` resync 渲染产物（`.agents/skills/**`）。
- 模板版本头双 locale 同步（过 `just check-sdd-templates`）。

## Capabilities

- `sdd-structured-skill-prompts`：仅模板内容变化，其规则（结构化提示协议
  必选节）不受影响，本 change 不改 live specs。

## Impact

- 受影响范围：apply skill 模板 × 2 locale + 渲染产物；无 CLI/合约/schema
  变化（`skip_specs_landing: true`）。
- 行为变化：agent 在 apply 期的 commit 频率从「每 task 一提交」收敛为
  「收尾单 commit」；main 历史从步骤日志变为语义单元。
- 与 `remove-sdd-status-command` 无文件交集（该 change 改
  draft/propose/apply-cycle 模板，本 change 只改 apply 模板），可并行推进。

## Open Questions

无——1B（change diff --summary）已被用户明确砍掉，本 change 只做 commit
纪律的模板指引。

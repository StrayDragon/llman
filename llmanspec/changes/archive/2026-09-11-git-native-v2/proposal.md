---
depends_on: []
blocks:
- lifecycle-self-expressive
branch: sdd/git-native-v2
base_sha: 0189a539a6da68dc280029bdf85862a320b63e6e
rules_touched:
- r1
- r111
- r124
- r130
- r137
- r135
checkpointed: true
checkpoint_sha: 0189a539a6da68dc280029bdf85862a320b63e6e
---

# Git-native 语义 v2：范围换锚 + rules_touched + show gateChecks

## Why

`specs-flatten-repo-run` 迁移实验暴露了流程层的两个系统性问题（见 `llmanspec/changes/archive/2026-09-11-specs-flatten-repo-run/`）：

1. **base_sha 锚定远端 + 范围语义过载**：`resolve_default_branch_ref` 优先 `origin/*`，在「长期本地工作、不 push」的真实用法下，每个 change 的 base 停在最后一次 push 的位置。锁定门禁（spec-format r135「对比 base_sha...HEAD 哈希集合」）、specs landing 判定（sdd-workflow r10）、`change diff`、staleness base 四个消费者共用这个漂移的 base——结果是零哈希漂移的机械迁移 change 也被迫 blanket `rules_edit_acked: true`，替上一个 change 的条文编辑背书。ack 的语义从「本 change 承认改了哪些规则」退化为「自上次 push 以来所有规则改动都由我担」。
2. **show 缺聚合门禁视图**：门禁散在 start/apply/finalize/archive 各处，agent 只能「走到哪被哪个命令拒绝才知道下一关」；`show --json` 已暴露 stage/readyToImplement/specsLanded，但缺「全部检查项 + 未过原因 + 下一步」的问路口径。

## What Changes

1. **范围语义统一换锚（D1）**：锁定门禁、specs landing 判定、`change diff`、staleness base 四处的 diff 范围一律改用**现算** `git merge-base <本地默认分支> HEAD`（本地 ref 优先，缺本地 ref 才回退 origin）；不再使用存储的 `binding.base_sha` 做范围。`base_sha` 降级为审计字段（读取兼容、写入保留、不参与任何范围计算）。效果：范围天然只含本分支工作，合并/rebase 后自动收缩，免疫未 push 积累——零新状态。
2. **`rules_edit_acked: bool` → `rules_touched: [<req-id>...]`（D2）**：锁定规则 ack 收窄到具体规则 id 列表，只豁免列表内的规则增删改。旧 bool 字段读取兼容（`rules_edit_acked: true` ≈ 全部 touched），新写入用 `rules_touched`。
3. **`show <change>` 增加聚合门禁视图（gateChecks）**：`--json` 增加 `gateChecks: [{name, pass, hint}]` 短键数组；text 模式末尾追加紧凑「Gates」段——**默认只输出通过计数一行 + 未过项（每项一行 name + hint）**，保证 token-efficiency（问路口径：「我到哪一步、什么在挡路」），不做全量内容倾倒。

## Capabilities / Impact

- `sdd-workflow`（r98 base_sha 语义、r10 landing 判定、r118 frontmatter 字段集、r134 门禁范围引用、checkpoint_sha 语义核对）
- `spec-format`（r135 锁定哈希门禁范围与 rules_touched）
- `cli` / `cli-experience`（show 的 JSON 键集与 text 输出口径——落点由实现者核对既有 show JSON 键的条文归属后定位）

## Non-goals

- 不引入 sidecar / 额外元信息文件（D3 否决：change 自表达，状态留在 proposal frontmatter）。
- 不做生命周期减法（start 自动 WIP commit、finalize 自动 commit、checkpoint 退役、draft/designed 合并）——方向已记录在 `llmanspec/changes/lifecycle-self-expressive/`（draft，二期）。
- 不改 pre-commit hook 配置（prek patch 冲突是外部工具行为）。

> 本 change 按仓库先例**停靠在 Designed**（不 `change start`）：Branch binding、Specs landing（r98/r10/r118/r134/r135 条文改写 + 新增 gateChecks/rules_touched 的 @executable 场景）与实现由实现者在 apply-cycle 的绑定分支上完成。

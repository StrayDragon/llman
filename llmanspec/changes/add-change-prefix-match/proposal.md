---
depends_on: []
branch: sdd/add-change-prefix-match
base_sha: 7c3e95692372740f17b87d65ebba52b894b687b6
checkpointed: false
checkpoint_sha: null
---

## Why

当前 `llman sdd show <change>`、`validate <change>`、`status <target>`、`graph <change>`、`change archive/attach/checkpoint/finalize/diff/delta <change>` 等所有需要 change 名参数的命令都只支持**精确匹配**（全名必须完全一致）。用户必须输入完整的 change id 才能操作。

当 change id 较长（如 `c123-fix-validate-hint-display-order`）时，输入完整 id 很繁琐。需要支持**前缀匹配**：输入前缀（如 `c123`）即可命中 `c123-fix-validate-hint-display-order`。

## What Changes

1. 在 `src/sdd/shared/discovery.rs` 新增 `resolve_change_id(root, input)` 公共函数：
   - 精确匹配 → `llmanspec/changes/<input>/proposal.md`（最高优先级）
   - 前缀匹配 → `llmanspec/changes/` 下目录名以 input 开头
   - 前缀匹配 → `llmanspec/changes/archive/` 下解出的 change id 以 input 开头
   - 唯一匹配自动返回；多匹配报错提示所有候选；无匹配报错
   - 结果缓存可选（但不必须）

2. 更新所有接收 change 名的命令使用该函数（而非直接目录存在检查或 exact contains）：
   - `show.rs` (`show_direct` → `show_change`)
   - `validate.rs` (`validate_direct` → `validate_by_type`，change 分支)
   - `status.rs` (`resolve_target` — 已有 contains 匹配，改为前缀优先 + 活性优先)
   - `graph.rs` (`build_seed_neighborhood`)
   - `change/archive.rs` (`run_with_root`)
   - `change/git_native.rs` (`run_attach` / `run_checkpoint` / `run_diff`)
   - `change/finalize.rs` (`run_finalize`)
   - `authoring/delta.rs`（所有接收 change_id 的函数）

3. 更新 `sdd-workflow` spec 添加新 requirement 文档化前缀匹配契约
4. 更新 `cli` spec 的 r42 扩展 target 解析规则
5. 添加可执行 `.feature` scenarios 验证前缀匹配行为

## Capabilities

- `sdd-workflow` — 新增 requirement（前缀匹配契约）
- `cli` — 扩展 r42（target 解析优先级规则）
- `sdd-bdd-mode-compat` — 不改（保证兼容性）

## Impact

- 小：所有 change 名查询命令的行为可感知变化（宽松了匹配规则）
- 低风险：向后兼容（精确匹配仍优先，已有 exact match 用户不受影响）
- 无需迁移：旧命令参数仍有效

## 附带范围（Incidental）

本 change 分支同时夹带了一个独立的 dead-code 清理 commit（`7eac336 refactor: clean up
dead code and trim dependencies`，~7700 行删除：`src/usage_stats/`、`src/x/cursor/`、
`src/x/codex/stats.rs`、`src/x/claude_code/stats.rs`、`src/sdd/project/partition_migrate.rs`
等）。这些删除与前缀匹配合约无关，属于独立的 dead-code 清理 concerns，因分支历史原因落
在本 change 的 diff 范围内（attach base_sha 为 `7c3e956`，而 `7eac336` 是其直接子提交）。
保留该 commit 以避免 rebase 改写已推送的历史；后续 dead-code 清理若需独立追溯，以
`7eac336` 为锚点。

## Design

见 `design.md`
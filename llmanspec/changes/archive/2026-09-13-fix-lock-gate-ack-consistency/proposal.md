---
depends_on: []
rules_touched: [r135]
branch: sdd/fix-lock-gate-ack-consistency
base_sha: f6df4dfe552f2235e756be5c80a7941e7be97bf9
---

## Why

issue #18：lock-gate 的确认集合与校验集合口径不一致，`--yes` 自写列表永远过不了自家门禁。三个缺陷均已在最小复现仓库中端到端复现：

1. **`--yes` 死锁**：`agent_marked_ids` 只扫当前工作树解析 `@agent` 标记（lock_gate.rs:221）。specs 压缩类变更删除了被确认的规则本身 → id 在工作树已不可见 → `--yes` 一个 id 都写不进 `rules_touched`，报错却仍指引 "pass --yes"，非交互流程死循环。
2. **哈希类编辑永久不可豁免**：`hashes_from_content` 建 id→hash 单向映射且 first-wins（lock_gate.rs:394）。base 有重复 `@req` 场景（同 id 不同内容）时，第二个哈希查不到 id，被报告为 `removed rule (<hash>)`——既违反 r135「报告 MUST 按 req-id 指明（不得只给哈希摘要）」，又因豁免分支只认 req-id 而对任何 `rules_touched` 内容免疫（手工补 id+哈希亦无法通过，已复现验证）。
3. **YAML 浮点坑**：`upsert_frontmatter_id_list` 经 `serde_saphyr::to_string` 输出形如 `20720420e754` 的裸标量，重解析被判 non-finite float，整个 frontmatter 解析炸掉（已复现验证；`locked_ack_for` fail-open 静默返回 None 雪上加霜）。
4. 附带：gate 错误行无计数，issue 实测单行 2123 个 token 无法评估规模。

## What Changes

- lock_gate 哈希映射反转为 **hash→id**（重复 req-id 的所有版本哈希挂回该 id；无 `@req` 的锁定场景编辑保持不可豁免并指引补标 `@req`；`rules_touched` 语义保持纯 req-id 列表，语义决策已经用户确认）。
- `@agent` 标记解析扩展到**有效范围 base 版本**：被本变更删除的规则按 base 侧标记参与 `--yes` 自动确认（`validate --yes` 单次豁免与 `finalize --yes` 写回两条路径共用该 helper，一并修复）。
- frontmatter 列表 upsert 启用 serde-saphyr `quote_all`（1.1.0 自带选项），杜绝类型重解释。
- lock-gate 错误行附带计数（`N undeclared, M exempted`）。
- spec-format r135 `@human` 文本锐化（标记解析口径 + hash→id 报告口径 + 计数要求），新增 2 个 `@executable` 场景钉住行为（`scenarios!` 编译期展开，步骤 fixture 在 apply 落地前 BDD 套件预期为红）。

## Capabilities

- spec-format（r135 锁定哈希门禁与收尾确认）

## Impact

- `crates/llman-sdd/src/sdd/change/lock_gate.rs`（主要实现）
- `tests/bdd_steps.rs`（2 个新 given fixture 步骤）
- `llmanspec/specs/spec-format.feature`（Specs landing：r135 文本 + 场景）
- 无破坏性合约变更：无字段/命令/tag 变更，`rules_touched`/`agent_acked` schema 不动，无 migrations。

# Design: fix-lock-gate-ack-consistency

## D1 豁免货币保持 req-id：hash→id 反查（决策已确认）

`hashes_from_content` 的第二返回值从 id→hash（first-wins，丢信息）改为 **hash→id**：
`ids_before.iter().find(|(_, h)| *h == hash)` 反查变成直接索引，重复 `@req` 场景的每个版本哈希都能挂回 id。

- 备选「rules_touched 接受哈希条目」（issue 期望 1 的字面形式）被否：需改 r135 合约语义，且用不透明哈希做人类确认货币违背「报告 MUST 按 req-id」的立法意图。
- 无 `@req` 的锁定场景编辑：保持不可豁免（violation），错误指引补标 `@req` 后重新走确认。当前 live specs 无此形态（已核），属防御性语义。
- 同哈希多 id（不同 id、相同内容）：BTreeMap or_insert 保留首个 id（BTreeMap 序），multiset 语义下豁免该编辑只需报告出的那一个 id，可接受。

## D2 @agent 标记解析 = base 版本 ∪ 工作树

`agent_marked_ids` 补充 base 侧解析：对 `changed_feature_files` 中每个文件，用 `hashes_at` 同款 `git show base:rel` 内容解析出「locked + @agent」的 req-id 集合，与工作树集合取并。实现形状：`hashes_from_content` 返回小结构体（hashes、hash→id、agent-marked ids）三件套，三处调用方共享，避免再复制一遍 parse 循环。

- `finalize --yes`（写回 frontmatter）与 `validate --yes`（单次豁免）共用该 helper，一次修复两处。
- 交互路径 `ack_all_undeclared` 现已正确（undeclared_ids 本就从 base 侧 diff 取 id），不动。

## D3 写入器 quote_all

`upsert_frontmatter_id_list` 改用 `serde_saphyr::to_string_with_options(.., SerializerOptions { quote_all: true, ..Default::default() })`。frontmatter 为机器管理的元数据，全引号无可读性代价，一劳永逸杜绝 YAML 类型重解释（含 `20720420e754` 浮点、`no`/`on` 布尔等）。

## D4 gate 错误行计数

`lock_gate_message` ERROR 文案在指引括号内追加 `{V} undeclared, {E} exempted`（V=violation 条目数，E=exempted 条目数）。INFO（全豁免）文案不加。可执行场景以 `stderr 包含 undeclared` 钉住。

## D5 测试接缝（seam）

- 行为合约：`llmanspec/specs/spec-format.feature` 新增 2 个 `@executable` 场景，复用既有 harness seam（CLI 子进程 `llman sdd validate`），新增 2 个 given fixture 步骤（`tests/bdd_steps.rs`，模式仿 `given_change_bound_editing_locked_rule`）：
  - `变更 {change} 绑定且删除了规则 {kind}`——绑定前先落一个（可选 `@agent` 标记的）新锁定场景并 commit（标记在 base），绑定后删除之；
  - `变更 {change} 绑定且删除了重复 @req 锁定场景之一`——绑定前给 sample 追加第二个 `@req:r1` 场景（不同内容）并 commit，绑定后删除其一。
- 实现细节：`lock_gate.rs` 单测扩展（hash→id 反查、agent base 解析、quote_all 往返）。

## 风险

- `scenarios!` 编译期展开：Specs landing 后、apply 落步骤前 `cargo test --features bdd` 为红（预期，propose 阶段门禁只跑 `--strict` 不跑 runner）。
- lock-gate 自举：本变更自身改 r135 锁定场景文本，proposal `rules_touched: [r135]` 自豁免（设计内流程）。

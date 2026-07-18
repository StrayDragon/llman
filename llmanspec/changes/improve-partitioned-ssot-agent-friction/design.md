# Design — improve-partitioned-ssot-agent-friction

## 背景

源自 `docs/release/partitioned-ssot/AGENT_FRICTION_PROMPT.md` 的 5 个操作性摩擦点。
Explore 阶段已查明真相（见 proposal.md），本设计聚焦两个合约层改动的设计决策。

## 设计决策

### D1: dual-write 错误消息格式（对应 #3a）

**现状**（`src/sdd/spec/partitioned.rs:253-259`）：
```
dual-write: N executable scenario(s) still have GWT in both spec.toon and .feature; run `llman sdd project migrate --kind partitioned`
```

**新格式**：
```
dual-write: N executable scenario(s) still have GWT in both spec.toon and .feature: [(r1, happy), (r12, login-ok)]; run `llman sdd project migrate --kind partitioned`
```

**决策**：
- 在 `partitioned.rs` 新增 helper `dual_write_pairs(doc, harness) -> Vec<(String, String)>`
  返回 `(req_id, scenario_id)` 对。现有 `dual_write_count` 改为 `pairs.len()` 的薄包装，
  避免逻辑重复。
- 格式化时用 Rust `format!` 拼接，每对 `(req_id, scenario_id)`，逗号空格分隔，方括号包裹。
- **不设上限**：双写量通常很小（迁移债务），全列出来更有利于一次性修完。若极端情况担心
  消息过长，可后续加 `... and K more`，但本次不做（YAGNI）。
- `compute_morphology` 的 `dual_write_count` 字段语义不变（仍是数量）。

**为什么不用 JSON 数组**：stderr 面向人类/agent 阅读，圆括号 + 方括号的 Rust-tuple-like
风格比 JSON 更易读，且和现有 `(req_id, scenario.id)` 文档措辞一致。

### D2: checkpoint 接受并忽略 --no-interactive（对应 #4）

**现状**：`src/sdd/command.rs:471-477` `Checkpoint` 只有 `--no-check`。

**决策**：
- 加 `#[arg(long)] no_interactive: bool` 字段。
- dispatch 时**完全忽略**该值（checkpoint 本身无 interactive 逻辑，`--no-check` 才是它的
  行为开关）。
- **不**实现真正的 interactive 模式——这是最小改动，满足「skill 统一传 `--no-interactive`」
  的需求。
- 不改 `change attach` / `change diff`（它们也不接受 `--no-interactive`，但 prompt 只提了
  checkpoint；若后续 agent 反馈 attach/diff 也需要，再单独开 change）。

**为什么接受并忽略而不是报错**：skill 脚本习惯对所有 change 子命令统一传 `--no-interactive`
（archive/freeze/migrate 都接受），checkpoint 报错打断 agent 流程。接受并忽略是
「principle of least surprise」+ 最小改动。

### D3: 非目标（明确排除）

- **不改 validate Totals 行**：`FAIL <item_type>/<id>` 已在 totals 行上方逐条打印
  （`validate.rs:1038-1045`），满足 prompt「列出失败 item id」的精神。Totals 行本身
  保持纯计数。skills 文档加 grep 指引即可。
- **不改 archived depends_on 解析**：代码已正确处理
  （`validation.rs:1616-1621`，archived 依赖报 INFO）。仅文档化。
- **不恢复 solidify / feature_delta**（prompt 明确 non-goal）。

## 影响的合约测试

- `llmanspec/specs/sdd-bdd-mode-compat/sdd-bdd-mode-compat.feature`：新增场景
  「双写错误消息列出具体冲突对」@req:r6，断言 `stderr 包含 (r1, happy)`。
- `llmanspec/specs/sdd-bdd-mode-compat/git-binding.feature`：新增场景
  「change checkpoint 接受 --no-interactive flag」@req:r57，断言
  `stderr 不含 unexpected argument`。
- `tests/sdd_bdd_compat_tests.rs`：smoke 列表若硬编码了 checkpoint 的 flag，需补
  `--no-interactive`（实施时确认）。

## 风险

- **低**：dual-write 消息格式变更是**追加**（保留 `dual-write:` 前缀和 `run ... migrate`
  后缀），现有断言 `stderr 包含 dual-write` 仍通过，只有新场景要求新子串。
- **低**：checkpoint 接受新 flag 是纯加法，不破坏现有行为。
- **中**：skills 文档改动若遗漏英文镜像会导致 locale parity 失败——实施时跑
  `just check-sdd-templates` 兜底。

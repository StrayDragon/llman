# Design: fix-migrate-toon-scenarios-conversion

## 现状

`src/sdd/project/migrate.rs::migrate_capability()`：

```rust
let merged = MainSpecDoc {
    ...
    requirements: toon_doc.requirements.clone(),
    scenarios: Vec::new(),   // ← 硬编码丢弃 scenarios[]
};
```

`parse_legacy_toon` 已正确解析 `scenarios[]` 为 `Vec<ScenarioEntry>`（含 `feature` 标志），但构造 `merged` 时被清零。后续 `dump_main_spec` 本就支持渲染 `@executable` 场景（`@req:<id> @executable` + 假如/当/那么、空列跳过、`feature:false` 跳过）——所以修复的核心是把 `scenarios` 传进去，外加配对守卫与记账。

## 方案

### 1. 转写（feature=true 行）

将 `toon_doc.scenarios` 传入 `merged.scenarios`。`dump_main_spec` 已具备：
- `@req:{sc.req_id} @executable` tag；
- `场景: {sc.id}` 标题；
- `假如/当/那么` 步骤，`""` 空列自动跳过（`if !value.is_empty()`）；
- `feature:false` 行自动跳过。

因此不需要额外的转写代码——`ScenarioEntry` 字段（req_id/id/given/when_/then_/feature）与 dump 渲染器天然对应。

### 2. 配对守卫

迁移产物须通过 `validate --strict`，其中对 `@executable` 有：
- 悬空 `@req:<rid>`（无匹配 `@human`）→ **ERROR**；
- 无 `@req` 的孤儿 → WARNING（strict 下升 ERROR）。

因此只转写 `req_id ∈ requirements` 的行。未配对 count 记入 `dropped_unpaired`。实测历史数据（29 个 capability 迁移前快照）100% 配对，该分支理论上不触发，但必须有显式记账兜底。

### 3. 记账分层

per-capability 报告字符串重组为（逗号分隔，与现有风格一致）：

```
wrote <path> (merged <M> feature file(s); converted_from_toon <C>; rules <N> enforced <E> manual <M> pending <P>; acceptance <A>; dropped <D>; dropped_notes <N>; dropped_unpaired <U>); removed legacy spec.toon
```

- `converted_from_toon` = 本次转写的 feature=true 行数；
- `dropped_notes` = `feature:false` 行数（note 行，非可执行）；
- `dropped` = 既有 `.feature` merge 时被过滤的块数（保留原语义）。
- `dropped_unpaired` = feature=true 但 req_id 不在 requirements 的行数（防御性记账，理论上 0）。

dry-run 亦补充 converted/dropped_notes 预估。

### 4. 幂等

不变：迁移后删除 `spec.toon` + merge 源 `.feature`，目标 `<cap>.feature` 保留；重跑 scan 阶段 `!dir.join(SPEC_FILE).exists()` → no-op。dump 渲染确定性（插入序 = requirements 序 + scenarios 序），无时间戳/随机。

### 5. 存量恢复（cli）

- 从 `a275d6a`（迁移前）提取 `llmanspec/specs/cli/spec.toon` 的 2 条 scenario：
  - `r112,baseline,已存在多个 active change 和 archived change,使用 change id 的前缀运行 llman sdd show/validate/status/graph/change archive,对应的完整 change 被找到且输出正确,true`
  - `r112,prefix-hint,活跃 change 中有 c123-fix-bug,用前缀 c123 运行 llman sdd show c123（以及 --json）,命令提示命中的完整 change（'c123' -> 'c123-fix-bug' (prefix match)），--json 输出含 matchedViaPrefix=true,true`
- 按转写规则手写 `@req:r112 @executable` 场景追加到 `llmanspec/specs/cli/cli.feature`（zh-CN 关键字，与 `dump_main_spec` 输出一致）。

## 测试策略

- **单元**：`src/sdd/project/migrate.rs` 测试 fixture 增加 feature=true 行（含空 given、feature=false note 行、未配对行），断言：
  - 产物含 `@req:r1 @executable` + `假如/当/那么`；
  - 空 given 不产生空 `假如` 行；
  - note 行不出现；
  - 报告计数 `converted_from_toon` / `dropped_notes` / `dropped_unpaired` 正确；
  - 迁移产物 `parse_main_spec` 通过、重跑 no-op、validate 语义（无悬空、无孤儿）。
- **集成（BDD）**：`spec-format.feature` r136 场景增加可执行用例：`spec.toon` 含 feature=true 行 + note 行 → migrate → stderr 含 `converted_from_toon`、产物含 `@executable`、`spec.toon` 不存在。`tests/bdd_steps.rs` 的 legacy-toon fixture 补 scenarios[]。
- **契约**：更新 `spec-format.feature` r136 @human 描述：转写 feature=true 行、记账 converted_from_toon/dropped_notes、配对守卫。
- **存量**：`cli.feature` 恢复 2 条 @executable 验收（挂 r112）。

## 风险

- `dump_main_spec` 对多行 step 值做 `split('\n') → join(" ")` 折叠：toon 的 GWT 单元格若含真实换行会被压平。可接受（确定性、合法 Gherkin 单行）。
- 既有 `dropped`（feature 块过滤）语义不变，避免误伤。
- 恢复 cli 验收后 `validate --strict` 须仍绿（r112 规则存在、无悬空、when/then 齐全）。
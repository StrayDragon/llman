---
depends_on: []
---

## Why

`llman sdd project migrate --kind toon2features` 在转写 legacy `spec.toon` 时静默丢弃了内嵌的 `scenarios[]` 表：

- `feature=true` 的可执行 GWT 行（given/when/then 列齐全）被整表丢弃，产物 `.feature` 里只有 850 条 `@human`，acceptance=0；
- 报告显示 `dropped 0`，`(merged 0 feature file(s); ...; acceptance 0; dropped 0)` —— **连「发生了丢弃」都不可见**；
- 三态计数失真：用户看到 enforced=0/pending=N，却不知道自己曾有验收素材。

本仓库自己就是受害者：`cli` 能力在 0.0.67 迁移时丢失了 2 条 `feature=true` 可执行场景（`baseline` / `prefix-hint`，挂 r112），当前 `llmanspec/specs/cli/cli.feature` 无任何 `@executable`。

## What Changes

1. **转写**：`feature=true` 的 toon scenario 行 → 单轨 `@executable` 场景：
   - `req_id` → `@req:<req_id>`（与同文件 `@human` 规则配对，防孤儿/悬空）；
   - `id` 列 → `场景:` 标题；
   - `given`/`when_`/`then_` → 假如/当/那么 步骤；空列跳过（不产生空步骤）。
2. **配对守卫**：只转写 `req_id` 存在于同文件 `requirements[]` 的行；未配对行计入 `dropped_unpaired`（显式记账）。实测历史数据 100% 配对。
3. **记账**：报告区分 `merged <N> feature file(s)` / `converted_from_toon <N>` / `dropped_notes <N>`；`dropped`（feature 文件级过滤）保留。任何丢弃都显式可见。
4. **幂等**：迁移后删除 `spec.toon`，重跑为 no-op；dump 输出确定性。
5. **存量恢复**：为本仓库 `cli` 能力恢复迁移中丢失的 2 条 `@executable` 验收场景（按转写规则手写落地到 `llmanspec/specs/cli/cli.feature`，等价于修复后的迁移产物）。

## Capabilities

- `spec-format`（r136 迁移合约）
- `cli`（存量恢复）

## Impact

- 修复后的迁移对既有存量迁移产物幂等（已迁移目录无 `spec.toon` 即 no-op）。
- `converted_from_toon` 仅在新迁移（仍有 `spec.toon`）时 > 0。
- 恢复 cli 验收场景后，r112 前缀匹配行为获得新的验收覆盖。
## Why

TOON `scenarios{req_id,id,given,when,then}` 和 Gherkin `场景: id / 假如 / 当 / 那么` 结构等价——都是 given-when-then 三元组。`.feature` 是 TOON scenarios 中**可执行子集**的 Gherkin 序列化，不是独立的行为规格。

当前 BDD-on 模型把两者当成割裂工件：
- `spec.toon` 退化到纯 meta（仅 kind/name/purpose），丢了 requirements 和 scenarios
- `.feature` 独立承载全部行为文本
- Delta 必须双轨（TOON ops + `.feature` 片段）
- Archive 整体覆盖 `.feature` → 冲突 → 必须手动 `--skip-specs`

## What Changes

### 1) spec.toon 结构回归，新增 `feature` 字段

BDD-on 的 `spec.toon` 恢复为完整结构，和 BDD-off **统一**：

```toon
kind: llman.sdd.spec
name: "errors-exit"
purpose: "..."
valid_scope[1]: llmanspec/specs/errors-exit
requirements[1]{req_id,title,statement}:
  r1,错误渲染,"CLI MUST render errors to stderr..."
scenarios[2]{req_id,id,given,when,then,feature}:
  r1,error-rendering,"...","...","...",true
  r1,internal-flow,"...","管理器扫描...","...",false
```

- `feature: true`（默认）：solidify 时写入 `.feature`
- `feature: false`：留在 `spec.toon` 作文档，不写入 `.feature`
- **框架无关**：solidify 不扫描 `tests/bdd_steps.rs`、不做 step pattern 匹配。`bdd.run_command` 负责运行时判定 step 是否存在——这是 `update-validate-bdd-auto-check` 已建立的职责分立。

### 2) 新增 solidify 阶段

`propose → apply → verify → solidify → archive → commit`

`solidify <change-id>` 对 delta 的每个 op_scenario：

| 条件 | 动作 |
|------|------|
| `feature: false` 显式 | SKIP（留在 toon） |
| `when` 含 `llman sdd validate\|archive\|solidify` | SKIP（递归风险） |
| 否则 | WRITE to `.feature` |

**BDD-off 时 `solidify <id>` 直接 no-op 通过。**

### 3) Delta 统一

propose 永远只产生 TOON delta（`ops` + `op_scenarios`），BDD-on/off 流程一致。

### 4) Archive 简化

移除 `copy_feature_files` / `find_feature_updates` 全部代码。Archive 只 merge `spec.toon`。

### 5) 新 CLI 命令 & skill

- `llman sdd solidify <change-id> [--dry-run]`
- `llman sdd project solidify-migrate [--dry-run]`
- `.agents/skills/llman-sdd-solidify/SKILL.md`

### 6) Skills 更新

propose/archive/compact/graph：移除 `.feature`/`feature_refs` 残留引导

## Capabilities

- `sdd-workflow`（pipeline 阶段、TOON 结构回归、archive 简化、delta 统一）
- `sdd-structured-skill-prompts`（solidify skill、skills 更新）

## Impact

- **破坏性变更**：BDD-on `spec.toon` 结构扩张，`solidify-migrate` 自动迁移
- `.feature` 不再是手工维护的工件，由 solidify + 显式 `feature` 字段控制
- 不引入框架绑定——solidify 只做过滤 + 序列化，不做 step pattern 分析

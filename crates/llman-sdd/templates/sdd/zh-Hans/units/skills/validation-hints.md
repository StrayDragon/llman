校验修复（单轨 feature-as-spec）：

1）缺少头注释（`missing # capability: header comment`）：
每个 capability `.feature`（`llmanspec/specs/<capability>.feature` 或 `llmanspec/specs/<capability>/<capability>.feature`）必须以以下注释开头：
```
# language: zh-CN
# capability: <capability>
# purpose: 一句话概述
# scope: src/
```

2）tag 语法（`@human constraint scenario must carry an @req:<req_id> tag` / `orphan acceptance scenario`）：
- 规则：`@req:<id> @human` —— statement 放场景描述（须含 MUST/SHALL）。
- 验收：`@executable` 且至少一个 `@req:<id>` 挂到规则。
- `@manual` 须与 `@human` 同用；禁止 `@human` 与 `@executable` 同场景。

3）遗留 `spec.toon`（`legacy spec.toon found ... run ... toon2features`）：
运行 `llman sdd project migrate --kind toon2features --yes`，审阅 diff 后提交。

Git-native 护栏：
- **Branch binding** → **Specs landing**：先 `change start` / `attach`，再在绑定的非默认分支编辑 live `.feature` 并 commit。
- 锁定规则：修改/删除既有 `@human` 场景以 WARNING 报告（报告制，r135/S0），不阻断 validate/finalize/diff。控制点：git 分支对比 + `llman sdd review` / `change diff` 报告浮现。确认元数据（`rules_touched` / `agent_acked` / `@agent` / `--yes` 锁定语义）已移除（q9 无兼容）。
- apply 前须 `readyToImplement=true`（或 `needs_specs_change: false`）。收尾优先 `change finalize`。
- 勿使用 `change delta` / solidify / `*.feature.delta.toon`。

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
- 锁定规则（报告制）：改/删既有 `@human` 场景只出 WARNING，不阻断 validate / change finalize / change diff；报告按 `@req:<id>` 指明被改的是哪条规则。控制点：git 分支对比 + `llman sdd review` / `change diff` 的报告浮现。旧的锁定确认元数据（frontmatter `rules_touched` / `agent_acked`、`@agent` tag、`--yes` 的确认语义）已全部删除，无别名、无兼容层。
- apply 前须 `readyToImplement=true`（或 `needs_specs_change: false`）。收尾优先 `change finalize`。
- 勿使用 `change delta` / solidify / `*.feature.delta.toon`。

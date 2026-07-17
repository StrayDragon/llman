# Design: Partitioned SSOT（一步到位）

## Ethics

- `ethics.risk_level`: high
- `ethics.prohibited_actions`: 不在未提供迁移命令时发布；不静默删下游 `.feature` 正文；BDD-off 路径不得回归
- `ethics.required_evidence`: compat BDD + `sdd_bdd_compat_tests` 全绿；本仓 `partition-migrate` 后 `validate --all --strict`；至少 1 条 `--check` harness 绿
- `ethics.refusal_contract`: 无法证明双写门禁与 migrate 可用时不得宣称可发布
- `ethics.escalation_policy`: 若 migrate 无法无损处理 Background/多 feature 文件，必须在 design 冻结规则并文档化

## Decision: 为何不是「toon 投影」也不是「纯 feature」

| 方案 | 否决理由 |
|---|---|
| toon SSOT + solidify 投影 | GWT×2；harness 与文字不同步风险 |
| 全面 feature-primary（含抽象 MUST） | 架构约束不适合 Gherkin；context 检索变差 |
| **Partitioned（本变更）** | 可执行进引擎；约束留 toon；内容零重叠；双管道 delta 碰不同字段 |

## `@req` 约定

- Gherkin 标签：`@req:<req_id>`，挂在 `场景:` / `Scenario:` 上（每场景至少一个；允许多 `@req`）
- locale 中英文关键字均支持解析
- 无标签的 harness 场景：validate WARNING（迁移期）→ 本仓 dogfood 结束后对本仓升 CRITICAL 可配置；**发布默认**：缺 `@req` = WARNING，双写 = CRITICAL under `--strict`

## feature_delta 文件格式

路径：`llmanspec/changes/<id>/specs/<capability>/<capability>.feature.delta.toon`

```toon
kind: llman.sdd.feature_delta
target: agent-runtime.feature
ops[2]{op,id,req_id,given,when,then}:
  add,react-terminates,ar1,"mock ...","运行 AgentRuntime","先执行工具再结束"
  modify,abort-drops-sse,ar12,"...","...","..."
  remove,ports-exist,ar5,,,
```

- `op`: add | modify | remove
- merge 键：scenario `id`（稳定）；`req_id` 用于写入/校验 `@req`
- remove 只删场景块，不删整个 feature 文件（若删空则保留 feature 头或删文件——实现选「无场景则删除文件」）

## solidify 新行为

1. 加载 change 的 toon delta + feature_delta（若有）
2. 对目标主 specs 跑与 validate 相同的链接/双写检查（change 作用域）
3. `--write-stubs`：仅为 feature_delta 的 `add` 且目标缺少该 id 时写入骨架（given/when/then 来自 delta）
4. **默认不**从 toon `op_scenarios` 投影全文（打破旧 r2）

Pipeline 建议：`propose → apply → verify → solidify → archive`

## Archive

1. Merge toon delta → `spec.toon`（现有）
2. Apply `*.feature.delta.toon` → `specs/<cap>/*.feature`
3. 冲突：modify/remove 找不到 id → 失败（非静默）
4. 不再整文件 `copy_feature_files` 覆盖（若仍有遗留路径，删除或仅在无 feature_delta 且显式 flag 时保留——本变更选择**删除覆盖复制**）

## context / index

- rebuild：可执行场景以 `.feature` 解析结果为准；toon 仅索引 `feature:false`（或无 feature 字段且无同 id feature 场景的文档场景）
- 碰撞：同 id 时 **feature 胜**，且不保留第二份 GWT
- `get_spec_content`：requirements 来自 toon；harness GWT 来自 feature 索引节点（可经 `@req` 挂到 req 下）

## partition-migrate 算法

对每个 `llmanspec/specs/<name>/`：

1. 解析 toon scenarios 中 `feature:true`（或缺省 true）行
2. 若 `.feature` 已有同 id：丢弃 toon 行 GWT，确保场景带 `@req:<req_id>`
3. 若 `.feature` 无该 id：追加场景块（GWT 自 toon）+ `@req`
4. 从 toon 删除这些可执行行（保留 `feature:false`）
5. `--dry-run` 打印计划

多 `.feature` 文件：按现有命名 `<name>.feature` 优先；其余 feature 只做 `@req` 补齐与双写检测，不把 toon 场景写入错误文件。

## 形态 JSON 草图

```json
{
  "name": "errors-exit",
  "morphology": {
    "constraintsReqCount": 2,
    "nonExecutableScenarioCount": 1,
    "harnessScenarioCount": 4,
    "reqLinkCoverage": 1.0,
    "dualWriteCount": 0
  }
}
```

## 发布 / 下游

- CHANGELOG：Breaking — BDD-on Partitioned SSOT
- 下游步骤：升级 llman → `sdd project partition-migrate` → `sdd validate --all --strict` → 修复缺 step → 试用反馈
- 本仓为第一狗粮：migrate 全部 30 spec

## 开放实现选择（apply 时可微调，不得削弱合约）

- feature_delta 文件名：`<cap>.feature.delta.toon` vs `feature.delta.toon`
- 缺 `@req` 默认 WARNING vs CRITICAL：发布默认 WARNING，文档说明下游可在下一 minor 升 CRITICAL

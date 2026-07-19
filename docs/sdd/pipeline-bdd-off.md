# SDD Pipeline — BDD-off

前提：`llmanspec/config.yaml` **不含** `bdd:` 段。约束与场景写在 change 内 TOON delta；archive 合并进主 `spec.toon`。

## Agent 如何选 skill

```mermaid
flowchart TD
  U[用户请求] --> A{意图清晰?}
  A -->|否| E[llman-sdd-explore]
  A -->|是| T{改 MUST/SHALL 或外部行为?}
  T -->|否| Q[llman-sdd-quick]
  T -->|是/不确定| P[llman-sdd-propose]
  E --> T
  P --> AP[llman-sdd-apply]
  AP --> V[llman-sdd-verify]
  V -->|CRITICAL| AP
  V -->|全绿| AR[llman-sdd-archive<br/>合并 TOON delta]
  AR --> C[git commit]
  Q --> C
  G[llman-sdd-graph] -.-> AP
  SC[llman-sdd-specs-compact] -.-> AR
```

## Delta / archive 闭环

```mermaid
flowchart LR
  N[change new] --> D[change delta<br/>skeleton / add-req / …]
  D --> IMP[实现代码 + tasks]
  IMP --> V[validate]
  V --> AR[change archive<br/>合并进主 spec.toon]
  AR --> GC[git commit]
```

不需要：feature 分支、`attach`、`checkpoint`、`finalize`、`.feature` harness（文件若存在，validate 在 BDD-off 下忽略）。

## 关键约束

- Delta 至少含一个 op + 匹配 scenario（含 MUST/SHALL）
- 托管 skill 的 `metadata.llman_sdd.bdd_mode` MUST 为 `off`
- Optional skills 默认不装；需 `extra_skills` 显式启用

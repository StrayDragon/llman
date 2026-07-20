# SDD Pipeline — BDD-on

前提：`llmanspec/config.yaml` 含 `bdd:` 段。可执行 GWT 在 live `*.feature`；约束在 live `spec.toon`。

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
  V -->|全绿| AR[llman-sdd-archive<br/>优先 change finalize]
  AR --> PR[本地 merge feature → 默认分支<br/>push / Hosting PR 可选]
  Q --> C[git commit]
  G[llman-sdd-graph] -.-> AP
  SC[llman-sdd-specs-compact] -.-> AR
```

## Git-native 闭环（实现侧）

```mermaid
flowchart LR
  B[非默认 feature 分支] --> N[change new]
  N --> L[编辑 live specs/features]
  L --> ATT[change attach]
  ATT --> IMP[实现代码 + tasks]
  IMP --> F[change finalize]
  F --> GC[一次 git commit]
  GC --> PR[本地 merge<br/>push / PR 可选]
```

Fallback（需严格 `checkpoint_sha = HEAD`）：`checkpoint` → commit → `archive` → commit。

## 关键约束

- 禁止在 main/master 上 propose/实现 BDD-on 变更
- 禁止 `change delta` / `*.feature.delta.toon` / solidify
- 托管 skill 的 `metadata.llman_sdd.bdd_mode` MUST 为 `on`；漂移时跑 `llman sdd init --update`

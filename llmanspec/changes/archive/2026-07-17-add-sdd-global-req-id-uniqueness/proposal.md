---
change_id: add-sdd-global-req-id-uniqueness
title: "全局唯一短 req_id 别名 + CLI 分配/解析 + 本仓批量去重"
status: proposal
priority: 1310
author: agent
track: SDD
---

# add-sdd-global-req-id-uniqueness

## Why

当前 `req_id` 仅在单个 capability 内唯一，本仓大量 `r1`/`r2` 跨 spec 复用，导致 `@req`、检索与 agent 引用歧义。

目标形态：

1. **`req_id` 是短别名**（`r12`、`rid`、或用户自定义 tag），在整个 `llmanspec/specs` 主库**全局唯一**——不必把 capability 编进 id。
2. **归属 / 展示 / 关联**由 `llman sdd` 子命令高效提供（resolve / list / show），agent 通过 CLI 取上下文，而不是靠长前缀 id 自描述。
3. **分配器** `next-req-id` 避免撞号；validate strict 默认拦跨 spec 重复。
4. 借本变更验收 **feature_delta** 形态。

## What Changes

### A) 全局唯一合约

- 范围：主库全部 `requirements[].req_id`
- **通用 validate 立即拦截**：`validate --all` / `validate <spec>` / `validate <change>`（加载主库时）发现跨 capability 重复 → **失败**（含默认与 `--strict`），不得静默放过
- 错误信息 MUST 含冲突 `req_id`、涉及 capability，并给出修复建议（`next-req-id` / `resolve-req`）
- `@req:X`：在**当前 feature 所属 capability** 的 toon 中解析 `X`；因全局唯一，跨库检索也可用同一 `X`

### B) 命名与迁移（短别名，非 capability 前缀）

- **保留短 id**：已全局唯一的保持不动；仅对冲突集合重新分配未占用短 id
- 默认自动分配形态：`r` + 正整数（扫描主库已用 `rN` 取最小空隙或 max+1——实现选定并在 help 声明）
- **允许自定义 tag**（如 `exit-nonzero`）：只要全局未占用即可；validate / add-req 同样守卫
- **不做** `{capability}--{old}` 嵌入式命名

### C) Authoring / 查询 CLI（agent 入口）

| 命令 | 作用 |
|---|---|
| `llman sdd spec next-req-id [--json]` | 打印一个当前未占用的短候选 id（默认 `rN`） |
| `llman sdd spec add-req …` | 给定 id 已全局占用 → 非零退出 |
| `llman sdd spec resolve-req <req_id> [--json]` | 解析归属：capability、title、statement、关联 harness 摘要 |

（`resolve-req` 是本变更的映射/展示面；list/show 可后续复用同一索引。）

### D) feature_delta 验收

可执行场景走 `*.feature.delta.toon`；约束说明留在 toon `feature:false`。

## Capabilities

- `sdd-bdd-mode-compat` — validate 全局唯一 + 强化 r6
- `sdd-workflow` — next-req-id / add-req 守卫 / resolve-req

## Impact

- **破坏性**：冲突的短 id 会被改写为新短 id；`@req` 同步更新；**不**引入长前缀
- 本仓 apply 批量去重后 `validate --all --strict` 必须绿
- 可用 git 随时回滚

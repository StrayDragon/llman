---
name: "llman-sdd-solidify"
description: "Partitioned SSOT：对 change 做 harness/约束一致性门禁（可选 --write-stubs）。在 apply 之后、archive 之前运行。"
metadata:
  version: "0.0.62"
---

# LLMAN SDD Solidify（Partitioned）

BDD-on 下 `.feature` 是可执行 harness 权威；`spec.toon` 是约束权威。solidify **不再**把 toon `op_scenarios` 全文投影覆盖 `.feature`。

## Pipeline

`apply → verify → solidify → archive`

## 硬约束

- BDD-off：no-op，提示 not configured。
- BDD-on：检查 `@req` 链接、双写、不可执行 id 入侵；失败则非 0。
- `--write-stubs`：仅对 `feature_delta` 的 **add** 且目标缺少该 scenario id 时写入骨架；**禁止**覆盖已有 GWT。
- 可执行场景变更应写 `*.feature.delta.toon`，不是双写进 toon scenarios。

## 命令

```bash
llman sdd solidify <change-id> [--dry-run] [--write-stubs]
```

成功时 stdout 含 `consistency ok`。

## 下游迁移

```bash
llman sdd project partition-migrate [--dry-run]
```

在执行之前，请先阅读 `llmanspec/config.yaml`，若其中包含 `context` 与 `rules` 请遵循。

常用命令：
- `llman sdd context --task "<description>" --paths "<files>"`（获取相关 specs）。使用 pageindex agentic 树检索后端（需配置 `LLMAN_SDD_INDEX_CHAT_MODEL`）。可用 `LLMAN_SDD_INDEX_BACKEND` 预设。
- `llman sdd list`（列出变更）
- `llman sdd list --specs`（列出 specs，含 purpose/scope 元数据）
- `llman sdd show <id>`（查看 change/spec）
- `llman sdd validate <id>`（校验变更或 spec）
- `llman sdd validate --all`（批量校验）
- `llman sdd index rebuild`（重建 pageindex 树索引——无需模型）
- `llman sdd index check`（检查索引新鲜度）
- `llman sdd archive run <id>`（归档变更）
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]`（冻结归档目录）
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]`（解冻归档）
- `llman sdd graph [CHANGE] [--format mermaid] [--scope active|archived|all] [--depth N]`（生成变更依赖图）

## Context
- 执行前先确认当前 change/spec 状态。
- 优先使用 `llman sdd context --task --paths` 获取相关 specs，而非全量读取或猜测。

## Goal
- 明确本次命令/skill 要达成的可验证结果。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。
- 在读取 spec 全文前，先使用 `llman sdd context --task --paths` 获取相关 specs。
- 判断变更规模后选择路径：行为合约变更走完整 SDD 流程，实现变更走快速路径。

## Workflow
- 以 `llman sdd` 命令结果为事实来源。
- 涉及文件/规范变更时执行校验。
- 首选 `llman sdd context` 获取相关 specs，而非全量读取或猜测。
- 当 context 不可用时，按错误提示处理（重建 index 或降级到 `list --specs --json`）。

## Decision Policy
- 高影响歧义必须先澄清。
- 已知校验错误下禁止强行继续。

## Output Contract
- 汇总已执行动作。
- 给出结果路径与校验状态。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。

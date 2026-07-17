---
name: "llman-sdd-solidify"
description: "Partitioned SSOT：对 change 做 harness/约束一致性门禁（可选 --write-stubs）。在 apply 之后、archive 之前运行。"
metadata:
  version: "{{ llman_version }}"
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

{{ unit("skills/sdd-commands") }}
{{ unit("skills/structured-protocol") }}

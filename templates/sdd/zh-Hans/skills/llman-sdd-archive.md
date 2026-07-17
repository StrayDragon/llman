---
name: "llman-sdd-archive"
description: "归档已完成的 llman SDD 变更。BDD-off 合并 TOON delta 到主 specs；BDD-on 在 attach/checkpoint 后仅封存 change 文档，再由 Git/PR merge 提升 live specs。在 verify 报告全绿后运行。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD 归档

使用此 skill 归档已完成的变更。**BDD-off**：合并 delta specs 到主 specs。**BDD-on**：仅移动 change 文档（specs 已在 feature 分支上 live），再经 Git/PR merge 提升。

## Pipeline 位置

```mermaid
flowchart LR
    verify["llman-sdd-verify<br/>验证"] --> archive
    archive["★ llman-sdd-archive ★<br/>归档（你现在在这里）"]
    archive --> commit["git commit<br/>完成闭环"]

    style archive fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 你现在在归档阶段：pipeline 最后一站。
> 📎 若 specs 逐渐膨胀，可运行 `llman-sdd-specs-compact` 压缩。

## 硬约束

- **必须先通过 verify 阶段全绿**：未通过验证的 change 禁止归档。
- **SSOT 校验**：每个 change 归档前必须通过 `llman sdd validate <id> --strict --no-interactive`。
- **不要问「要不要继续」**：批量归档时间线上一路执行到底，除非遇到无法自动解决的错误。

## 步骤

### 0) Preflight
- `git status --porcelain`：确认工作区改动属于已完成的 change。
- 若有未预期改动，先处理（stash 或报告）。

### 1) 确认目标变更
- 确定目标 ID：单个或批量（来自用户输入或 `llman sdd list --json`）。
- 始终说明："归档 IDs：<id1>, <id2>, ..."。
- 确认每个 change 都已通过 verify 阶段的全绿验证。

### 2) 逐个归档
- 先逐个校验：`llman sdd validate <id> --strict --no-interactive`。
- 校验失败 → STOP 并报告；不要跳过校验强行归档。
- 可选预览：`llman sdd change archive <id> --dry-run`。
- 执行归档：
  - 默认：`llman sdd change archive <id>`
  - 仅工具类变更：`llman sdd change archive <id> --skip-specs`
  - **任一失败立即停止**，报告剩余未处理 ID。
- **BDD-on（Git-native Partitioned SSOT）**：
  - 前置：已 `llman sdd change attach <id>`，再 `llman sdd change checkpoint <id>`（干净工作区 + 门禁），仍在 feature 分支上。
  - `change archive` **只移动 change 文档**到 `changes/archive/`——**不会**把 TOON delta 当 SSOT 合并，也永不 apply `feature_delta`。
  - change 下遗留活跃 `*.feature.delta.toon` 是迁移阻断项——归档前须移除/迁移。
  - 归档后，通过正常 Git/PR 将 feature 分支合并进默认分支，以提升 live `llmanspec/specs/**`。
- **BDD-off**：
  - `change archive` 按今日流程将 change 内 TOON delta 合并进主 `spec.toon`。
  - 不要求 attach / checkpoint / feature 分支 / harness。

### 3) 全量校验
- 全部归档完成后执行：`llman sdd validate --all --strict --no-interactive`。
- 确认归档后的 specs 工件一致。

### 4) Commit / merge 引导
- BDD-off：输出建议 commit message（格式：`feat(sdd): archive <id1>, <id2> - <简短总结>`），然后 `git add -A && git commit -m "..."`。
- BDD-on：文档归档后，打开/合并 feature 分支 PR，使 live specs/features 进入默认分支。
- 若用户要求自动 commit 归档文档提交，执行后输出 commit hash。

> 💡 上一阶段 `llman-sdd-verify`（验证通过）→ 本阶段归档后闭环结束。若 specs 逐渐膨胀，可运行 `llman-sdd-specs-compact` 压缩。

{{ unit("workflow/archive-freeze-guidance") }}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

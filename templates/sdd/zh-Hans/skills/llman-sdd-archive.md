---
name: "llman-sdd-archive"
description: "归档已完成的 llman SDD 变更。自动 ff-merge 到默认分支，再将 change 文档改名到 archive/。在 verify 报告全绿后运行。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD 归档

使用此 skill 归档已完成的变更。前置：verify 全绿，且变更已 Branch binding、Specs landing 完成（或 `skip_specs_landing`；归档时 live specs 已在绑定分支上）。archive/finalize **自动 ff-merge** 到默认分支，**再将** change 文档改名到 `changes/archive/`（脏改名留一次 `git commit`）。`git push` / Hosting PR 仅为可选。

## Pipeline 位置

```mermaid
flowchart LR
    verify["llman-sdd-verify<br/>验证"] --> archive
    archive["★ llman-sdd-archive ★<br/>归档（你现在在这里）"]

    style archive fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 你现在在归档阶段：Git-native 生命周期的最后一站。
> 📎 若 specs 逐渐膨胀，可运行 `llman-sdd-specs-compact` 压缩。

## 硬约束

- **必须先通过 verify 阶段全绿**：未通过验证的 change 禁止归档。
- **须已 Branch binding**：`change start` / `attach` 已完成；无绑定则 STOP。
- **SSOT 校验**：每个 change 归档前必须通过 `llman sdd validate <id> --strict --no-interactive`。
- **不要问「要不要继续」**：批量归档时间线上一路执行到底，除非遇到无法自动解决的错误。
- **收尾不默认导向 PR/push**：archive/finalize 后由 CLI 处理本地 ff-merge，再一次性 `git commit` 提交文档改名。`git push` / Hosting PR 仅为可选——仅当用户或项目明确要求远程审查时才做。**Agent MUST NOT** 因本 skill 默认执行 push 或创建 PR。

## 步骤

### 0) Preflight
- `git status --porcelain`：确认工作区改动属于已完成的 change。
- 若有未预期改动，先处理（stash 或报告）。

### 1) 确认目标变更
- 确定目标 ID：单个或批量（来自用户输入或 `llman sdd list --json`）。
- 始终说明："归档 IDs：<id1>, <id2>, ..."。
- 确认每个 change 都已通过 verify 阶段的全绿验证。

### 2) 逐个归档
- **人审检查点（每个 id 归档执行前，含批量）**：运行 `llman sdd review --capability <id>`。退出码为零 → 继续；非零 = CRITICAL 发现：STOP 修复后重跑；MUST NOT 带着 CRITICAL 归档。
- 先逐个校验：`llman sdd validate <id> --strict --no-interactive`。
- 校验失败 → STOP 并报告；不要跳过校验强行归档。
- 可选预览：`llman sdd change archive <id> --dry-run`。
- 执行归档：
  - 默认：`llman sdd change archive <id>`
  - 仅工具类变更：`llman sdd change archive <id> --skip-specs`
  - **任一失败立即停止**，报告剩余未处理 ID。
- **Git-native 收尾**：
  - 前置：已 Branch binding（`change start` / `attach`）；仍在绑定分支上（或 ff-merge 后已在默认分支）。
  - `change archive` / `change finalize` **先自动 ff-merge**（`git merge --ff-only <feature>` 到默认分支），**再**将 change 文档改名到 `changes/archive/`——merge 失败也不会回滚改名。
  - specs 下遗留 `*.feature.delta.toon` 或 `spec.toon` 均为迁移阻断项——跑 `llman sdd project migrate --kind toon2features`。
  - **推荐：单 commit 收尾（`change finalize`）**——同进程跑门禁 → 自动 ff-merge → 文档改名；结束后工作区脏一次，**一次 `git commit`** 收尾：
    ```text
    1. 实现 live specs + 代码（工作区可保持脏）
    2. llman sdd change finalize <id>   # 门禁 + ff-merge + 文档改名
    3. git commit                       # 一次提交：实现 + frontmatter + archive 改名
    ```
    **`checkpoint_sha` 语义**：finalize 写入的是 attach 时的 `base_sha`，不是实现 commit 的 HEAD（单 commit 模式下实现 commit 尚未发生）。如需精确指向实现 commit，走下方 fallback。
  - **Fallback：多 commit 时序（`checkpoint` + `archive`）**——需要严格 `checkpoint_sha`、或想中途 review 实现快照时使用：
    ```text
    1. git commit   # 提交 live specs + 代码（让工作区干净，checkpoint 才能跑）
    2. llman sdd change checkpoint <id>   # 写入 checkpointed / checkpoint_sha（指向实现 commit HEAD）
    3. git commit   # 提交 proposal.md 的 checkpoint 元数据
    4. llman sdd change archive <id>      # ff-merge + 文档改名
    5. git commit   # 提交 archive 改名
    ```

### 3) 全量校验
- 全部归档完成后执行：`llman sdd validate --all --strict --no-interactive`。
- 确认归档后的 specs 工件一致。

### 4) Commit 引导
- 输出建议 commit message（格式：`feat(sdd): archive <id1>, <id2> - <简短总结>`），若尚未提交则 `git add -A && git commit -m "..."`。
- 可选：ff-merge 后 `git branch -d <feature>`。push / Hosting PR 仅在用户或项目明确要求远程审查时才做。
- 若用户要求自动 commit 归档文档提交，执行后输出 commit hash。
- **archived `depends_on`**：archive 会把 change 目录改名为 `archive/YYYY-MM-DD-<id>`，但 validate 会把指向 archived/frozen id 的 `depends_on` 识别为 INFO（非 ERROR），所以**归档后无需**手动更新其它 change 的 `depends_on` frontmatter。

> 💡 上一阶段 `llman-sdd-verify`（验证通过）→ 本阶段归档后闭环结束。若 specs 逐渐膨胀，可运行 `llman-sdd-specs-compact` 压缩。

{{ unit("workflow/archive-freeze-guidance") }}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

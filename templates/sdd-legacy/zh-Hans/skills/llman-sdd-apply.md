---
name: "llman-sdd-apply"
description: "实施一个 llman SDD 变更的 tasks，并同步更新 tasks.md 勾选状态。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD Apply

使用此 skill 按顺序完成 `llmanspec/changes/<id>/tasks.md`，直到完成或受阻。

## 步骤
1. 选择变更 id：
   - 若已提供，直接使用。
   - 否则先从上下文推断；若不明确，运行 `llman sdd-legacy list --json` 并让用户选择。
   - 始终说明："使用变更：<id>"，并告知如何覆盖。
2. 检查前置条件：
   - 必须存在：`llmanspec/changes/<id>/tasks.md`
   - 若缺失，建议先用 `/llman-sdd:continue <id>`（或 `/llman-sdd:ff <id>`）补齐规划工件，然后 STOP。
3. 阅读上下文文件（视情况而定）：
   - `llmanspec/changes/<id>/proposal.md`
   - `llmanspec/changes/<id>/design.md`（如存在）
   - `llmanspec/changes/<id>/tasks.md`
   - `llmanspec/changes/<id>/specs/**`
4. 展示状态：
   - 进度："N/M tasks complete"
   - 接下来 1–3 个未完成任务（简短概览）
5. 按顺序实施 tasks：
   - 改动保持最小并严格围绕当前任务
   - 完成一项任务后立刻更新 checkbox（`- [ ]` → `- [x]`）
   - 若任务不明确、遇到阻塞、或发现 specs/design 与现实不一致，必须 STOP 并询问用户下一步。
6. 在完成（或暂停）时运行校验：
   ```bash
   llman sdd-legacy validate <id> --strict --no-interactive
   ```
   - 若校验无误，建议 `/llman-sdd:verify <id>` 与 `/llman-sdd:archive <id>`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

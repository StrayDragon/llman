---
name: "llman-sdd-apply"
description: "实施一个 llman SDD change 的 tasks，并同步更新 tasks.md 勾选状态。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD Apply

使用此 skill 按顺序实施某个 change 的 `tasks.md`。

## 步骤
1. 选择 change id：
   - 若已提供，直接使用。
   - 否则运行 `llman sdd list --json` 并让用户选择。
2. 阅读上下文文件（视情况而定）：
   - `llmanspec/changes/<id>/proposal.md`
   - `llmanspec/changes/<id>/design.md`（如存在）
   - `llmanspec/changes/<id>/tasks.md`
   - `llmanspec/changes/<id>/specs/**`
3. 按顺序实施 tasks：
   - 改动保持最小并严格围绕当前任务
   - 完成一项任务后立即更新 checkbox（`- [ ]` → `- [x]`）
4. 若任务不明确或遇到阻塞，STOP 并询问用户下一步。
5. 在完成（或暂停）时运行校验：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#structured-protocol}}

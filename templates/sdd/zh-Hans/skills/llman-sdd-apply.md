---
name: "llman-sdd-apply"
description: "实施一个 llman SDD 变更的 tasks，并同步更新 tasks.md 勾选状态。"
---

# LLMAN SDD Apply

使用此 skill 按顺序完成 `llmanspec/changes/<id>/tasks.md`，直到完成或受阻。

## 步骤
1. 选择变更 id：
   - 若已提供，直接使用。
   - 否则先从上下文推断；若不明确，运行 `llman sdd list --json` 并让用户选择。
   - 始终说明："使用变更：<id>"，并告知如何覆盖。
2. 检查前置条件（权威阶段守卫）：
   - 从权威来源读取变更阶段：
     ```bash
     stage=$(llman sdd show <id> --json --type change | jq -r .stage)
     ```
     （若无 `jq`，可用任意工具从 JSON 中解析 `stage` 值。）
   - 若 `stage` 不为 `full`，变更尚未准备好被实现 → 必须停止并给出守卫提示：
     - `draft`："变更 <id> 是 draft 提案（仅 proposal.md），尚未准备好被实现。请先用 llman-sdd-continue <id> 把它长大到 full（proposal → specs → design → tasks）。"
     - 其他非 full 阶段（`specified`/`designed`）："变更 <id> 处于 <stage> 阶段，尚未准备好被实现。请先用 llman-sdd-continue <id> 长大到 full。"
   - `full` 阶段意味着 `tasks.md` 已存在，继续。
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
{% if bdd_enabled %}
6. **BDD 回归**:
   - 每完成一个 task，运行关联的 BDD 测试确保不回退:
     `{{ bdd_run_command }}`
   - 如有 scenario 由 PASS 变 FAIL，立即停止并报告
{% endif %}
7. 在完成（或暂停）时运行校验：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```
   - 若校验无误，建议运行 `llman-sdd-verify`，然后执行归档：`llman sdd archive run <id>`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

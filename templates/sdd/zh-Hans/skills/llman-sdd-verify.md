---
name: "llman-sdd-verify"
description: "验证实现是否与 llman SDD 的 specs/design 一致，并给出最小修复建议。"
---

# LLMAN SDD Verify

使用此 skill 验证实现是否与该 change 的 artifacts 一致。

## 步骤
1. 确定 change id（不明确时让用户从 `llman sdd list --json` 选择）。
2. 检查阶段守卫（权威）：
   ```bash
   stage=$(llman sdd show <id> --json --type change | jq -r .stage)
   ```
   （若无 `jq`，可用任意工具从 JSON 中解析 `stage` 值。）
   - 若 `stage` 不为 `full`，变更尚未实现、无可验证内容 → 必须停止并给出守卫提示：
     - `draft`："变更 <id> 是 draft 提案（仅 proposal.md），尚无可验证的实现。请先用 llman-sdd-continue <id> 长大到 full，再用 llman-sdd-apply <id> 实现。"
     - 其他非 full 阶段（`specified`/`designed`）："变更 <id> 处于 <stage> 阶段，尚未准备好被验证。请先长大到 full 并实现。"
3. 先跑一个快速校验门禁：
   - `llman sdd validate <id> --strict --no-interactive`
4. 阅读：
   - `llmanspec/changes/<id>/specs/` 下的 delta specs
   - `proposal.md` 与 `design.md`（如存在）
   - `tasks.md`（理解实现范围）
5. 对比 artifacts 与代码：
   - 标出不一致（缺失行为、错误行为、缺测试/文档）
   - 给出最小修复建议或建议更新 artifacts
{% if bdd_enabled %}
6. **BDD 验证**:
   - 读取 delta specs 中关联的 feature_refs
   - 对每个 scope=acceptance 且 required=true 的 .feature 文件:
     - 执行: `{{ bdd_run_command }}`（替换 {feature_name} 为实际 feature 名）
     - 所有 scenario MUST 通过
     - 失败的 scenario 映射到对应 requirement ID，标记为 CRITICAL
{% if bdd_verify_prompt %}
   - 额外要求: {{ bdd_verify_prompt }}
{% endif %}
{% endif %}
7. 输出简短报告：
   - **CRITICAL**（归档前必须修复）
   - **WARNING**（建议修复）
   - **SUGGESTION**（可选优化）
8. 若存在 CRITICAL，建议用 `llman-sdd-apply` 修复；若通过则建议归档：`llman sdd archive run <id>`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

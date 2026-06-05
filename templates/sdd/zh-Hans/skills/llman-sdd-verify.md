---
name: "llman-sdd-verify"
description: "验证实现是否与 llman SDD 的 specs/design 一致，并给出最小修复建议。"
---

# LLMAN SDD Verify

使用此 skill 验证实现是否与该 change 的 artifacts 一致。

## 步骤
1. 确定 change id（不明确时让用户从 `llman sdd list --json` 选择）。
2. 先跑一个快速校验门禁：
   - `llman sdd validate <id> --strict --no-interactive`
3. 阅读：
   - `llmanspec/changes/<id>/specs/` 下的 delta specs
   - `proposal.md` 与 `design.md`（如存在）
   - `tasks.md`（理解实现范围）
4. 对比 artifacts 与代码：
   - 标出不一致（缺失行为、错误行为、缺测试/文档）
   - 给出最小修复建议或建议更新 artifacts
{% if bdd_enabled %}
5. **BDD 验证**:
   - 读取 delta specs 中关联的 feature_refs
   - 对每个 scope=acceptance 且 required=true 的 .feature 文件:
     - 执行: `{{ bdd_run_command }}`（替换 {feature_name} 为实际 feature 名）
     - 所有 scenario MUST 通过
     - 失败的 scenario 映射到对应 requirement ID，标记为 CRITICAL
{% if bdd_verify_prompt %}
   - 额外要求: {{ bdd_verify_prompt }}
{% endif %}
{% endif %}
6. 输出简短报告：
   - **CRITICAL**（归档前必须修复）
   - **WARNING**（建议修复）
   - **SUGGESTION**（可选优化）
7. 若存在 CRITICAL，建议用 `llman-sdd-apply` 修复；若通过则建议归档：`llman sdd archive run <id>`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

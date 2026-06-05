---
name: "llman-sdd-verify"
description: "Verify implementation matches llman SDD specs/design and propose fixes."
---

# LLMAN SDD Verify

Use this skill to verify that the implementation matches the change's artifacts.

## Steps
1. Select the change id (or ask the user to pick from `llman sdd list --json`).
2. Run a fast validation gate:
   - `llman sdd validate <id> --strict --no-interactive`
3. Read:
   - Delta specs under `llmanspec/changes/<id>/specs/`
   - `proposal.md` and `design.md` if present
   - `tasks.md` to understand what was implemented
4. Compare artifacts vs code:
   - Identify mismatches (missing behavior, wrong behavior, missing tests/docs)
   - Suggest minimal fixes or artifact updates
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
6. Produce a short report:
   - **CRITICAL** (must fix before archive)
   - **WARNING** (should fix)
   - **SUGGESTION** (nice to have)
7. If CRITICAL exists, suggest `llman-sdd-apply`. If clean, suggest archive: `llman sdd archive run <id>`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

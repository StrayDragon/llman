---
name: "llman-sdd-verify"
description: "Verify implementation matches llman SDD specs/design and propose fixes."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Verify

Use this skill to verify that the implementation matches the change's artifacts.

## Steps
1. Select the change id (or ask the user to pick from `llman sdd list --json`).
2. Check the stage gate (authoritative):
   ```bash
   stage=$(llman sdd show <id> --json --type change | jq -r .stage)
   ```
   (If `jq` is unavailable, parse the `stage` value from the JSON with any tool.)
   - If `stage` is not `full`, the change has nothing implemented to verify → STOP with a guard:
     - `draft`: "Change <id> is a draft proposal (proposal.md only); nothing to verify yet. Grow it to full first with: llman-sdd-continue <id>, then implement with llman-sdd-apply <id>."
     - other non-full (`specified`/`designed`): "Change <id> is in <stage> stage, not ready to verify. Grow it to full and implement first."
3. Run a fast validation gate:
   - `llman sdd validate <id> --strict --no-interactive`
4. Read:
   - Delta specs under `llmanspec/changes/<id>/specs/`
   - `proposal.md` and `design.md` if present
   - `tasks.md` to understand what was implemented
5. Compare artifacts vs code:
   - Identify mismatches (missing behavior, wrong behavior, missing tests/docs)
   - Suggest minimal fixes or artifact updates
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
7. Produce a short report:
   - **CRITICAL** (must fix before archive)
   - **WARNING** (should fix)
   - **SUGGESTION** (nice to have)
8. If CRITICAL exists, suggest `llman-sdd-apply`. If clean, suggest archive: `llman sdd archive run <id>`.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

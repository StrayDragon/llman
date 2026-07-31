---
name: "llman-sdd-validate"
description: "校验 llmanspec 变更与 specs 并提供修复提示。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD 校验

使用此 skill 校验变更/spec 格式与过期状态。

## 步骤
1. 校验单个条目：`llman sdd validate <id>`。
2. 批量校验：`llman sdd validate --all`（或 `--changes` / `--specs`）。
3. 在 CI 或自动化场景中使用 `--strict` 与 `--no-interactive`。
4. 若校验失败，汇总错误并给出最小、可执行的修复建议。
{% if bdd_enabled %}
5. **BDD 校验（Git-native Partitioned SSOT）**：
   - 在**绑定分支**上验证 live `.feature` Gherkin 与 `@req` / 双写门禁（须已 Branch binding）。
   - `.feature` 是 harness 权威——可执行 GWT 只在 live `.feature` 维护（无 solidify；无 `feature_delta` / `change delta`）。
   - Change 生命周期门禁：`change start` / `attach`（Branch binding）、`finalize`（推荐）/ `checkpoint`（fallback）/ `diff`（只读）。
   - `llman sdd validate --specs` 默认自动运行 `bdd.run_command`。
   - 可用 `list --specs --json` 查看 `morphology`（含 `dualWriteCount`）。
   - Change JSON 状态字段：`stage` / `specsLanded` / `readyToImplement`（`show --json`）。
{% endif %}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/ethics-governance") }}

---
name: "llman-sdd-validate"
description: "校验 llmanspec 变更与 specs 并提供修复提示。"
metadata:
  version: "{{ llman_version }}"
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
   - 在 feature 分支上验证 live `.feature` Gherkin 与 `@req` / 双写门禁。
   - `.feature` 是 harness 权威——可执行 GWT 在此维护（无 solidify；无 `feature_delta`）。
   - Change 生命周期门禁：`llman sdd change attach` / `finalize`（推荐）/ `checkpoint`（fallback）/ `diff`（diff 只读）。
   - `llman sdd validate --specs` 默认自动运行 `bdd.run_command`。
   - 可用 `list --specs --json` 查看 `morphology`（含 `dualWriteCount`）。
{% endif %}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

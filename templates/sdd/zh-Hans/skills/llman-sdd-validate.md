---
name: "llman-sdd-validate"
description: "校验 llmanspec 变更与 specs 并提供修复提示。"
---

# LLMAN SDD 校验

使用此 skill 校验变更/spec 格式与过期状态。

## 步骤
1. 校验单个条目：`llman sdd validate <id>`。
2. 批量校验：`llman sdd validate --all`（或 `--changes` / `--specs`）。
3. 在 CI 或自动化场景中使用 `--strict` 与 `--no-interactive`。
4. 若校验失败，汇总错误并给出最小、可执行的修复建议。
{% if bdd_enabled %}
5. **BDD 校验**:
   - 检查 feature_refs 引用的 .feature 文件存在
   - 验证 .feature 文件语法合法（gherkin 解析，语言: {{ bdd_default_language | default("en") }}）
   - 报告 scenario 覆盖率（.feature 中 scenario 数 vs spec 中 scenario 数）
{% endif %}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

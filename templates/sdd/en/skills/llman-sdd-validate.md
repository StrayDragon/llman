---
name: "llman-sdd-validate"
description: "Validate llmanspec changes and specs with actionable fixes."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Validate

Use this skill to validate change/spec format and staleness.

## Steps
1. Validate one item: `llman sdd validate <id>`.
2. Validate all: `llman sdd validate --all` (or `--changes` / `--specs`).
3. Use `--strict` and `--no-interactive` for CI-like checks.
4. If validation fails, summarize the errors and propose minimal, concrete fixes.
{% if bdd_enabled %}
5. **BDD 校验**:
   - 验证 spec 目录下 .feature 文件语法合法（gherkin 解析）。
   - `.feature` 由 `llman sdd solidify` 自动生成——不要手动编辑。
   - `llman sdd validate --specs` 默认自动运行 `bdd.run_command`。
   - 报告 scenario 覆盖率（.feature 中 scenario 数 vs spec 中 scenario 数）
{% endif %}

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

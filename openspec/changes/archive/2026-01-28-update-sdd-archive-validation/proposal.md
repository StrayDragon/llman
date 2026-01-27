## Why
- 目前 `llman sdd archive` 不会在归档前执行与 `sdd validate` 等价的校验，staleness 或 frontmatter 问题可能被直接合并进主 specs。
- 现有 `llman sdd update-skills` 生成的 SKILL.md frontmatter 不符合 Agent Skills 规范（name/description/目录一致性），影响自动化校验与生态兼容性。

## What Changes
- 归档前对本次涉及的 specs 进行严格校验（含 staleness），校验失败即中止归档。
- 提供显式绕过参数 `--force`，但不在 SDD skills 模板中暴露，并避免在失败提示中引导绕过。
- 规范化 SDD skills 输出的 SKILL.md frontmatter 以符合 Agent Skills 规范，同时保留 `llman-template-version` 元信息。

## Impact
- 受影响规范：`sdd-workflow`
- 受影响代码：`src/sdd/archive.rs`、`src/sdd/command.rs`、`src/sdd/validation.rs`、`src/sdd/staleness.rs`
- 受影响模板：`templates/sdd/**/skills/*.md`
- 可能新增/调整校验：`scripts/check-sdd-templates.py`、相关测试

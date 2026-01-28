## Why
- Agent Skills 规范推荐将扩展属性放在 `metadata` 字段中，以避免顶层 frontmatter 扩展污染。
- 当前 SDD skills 模板在 frontmatter 顶层放置 `llman-template-version`，与规范最佳实践不一致。

## What Changes
- 将 `llman-template-version` 放入 frontmatter 的 `metadata` 字段（仅对带 YAML frontmatter 的模板）。
- 更新模板检查脚本以从 `metadata` 读取版本，并校验一致性。

## Impact
- 受影响规范：`sdd-workflow`
- 受影响文件：`templates/sdd/**/skills/*.md`、`scripts/check-sdd-templates.py`

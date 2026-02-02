## 1. Implementation
- [x] 1.1 更新 skills 配置解析为 v2（targets-only），提供清晰的迁移错误提示
- [x] 1.2 调整扫描逻辑为仅扫描 `<skills_root>`，遵循忽略规则并允许技能软链接
- [x] 1.3 移除 `store/` 快照流程，更新 registry 结构与写入逻辑
- [x] 1.4 重构同步：link 从 `<skills_root>/<skill_id>` 出发（移除 copy）
- [x] 1.5 重做交互流程：多选技能 → 选择目标 → 确认同步
- [x] 1.6 统一目标冲突处理（覆盖/跳过），非交互使用 `--target-conflict`
- [x] 1.7 更新文档与提示文案

## 2. Tests & Validation
- [x] 2.1 覆盖 config v2、软链接扫描、目标冲突处理的测试
- [x] 2.2 `openspec validate refactor-skills-single-root --strict --no-interactive`

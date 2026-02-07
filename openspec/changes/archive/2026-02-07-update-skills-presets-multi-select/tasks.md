# Tasks: update-skills-presets-multi-select

## 1. 交互入口与预设整合

- [x] 1.1 移除 `Select mode` 菜单并恢复直接 `agent -> scope -> skills`
- [x] 1.2 将配置化 presets 以分组节点并入 skills 多选
- [x] 1.3 将目录分组自动映射为分组节点
- [x] 1.4 选择分组节点时自动展开为对应技能集合并去重
- [x] 1.5 保持取消或空选择为 no-op
- [x] 1.6 用 `ratatui` 实现 skills 三态交互（含 partial 状态）
- [x] 1.7 支持在树形技能选择中按关键字过滤搜索

## 2. 技能展示文案

- [x] 2.1 交互技能项展示 `skill_id (directory_name)`
- [x] 2.2 保持目录名来源优先于 frontmatter name

## 3. 测试与验证

- [x] 3.1 单测覆盖分组批量选择与默认索引逻辑
- [x] 3.2 `just qa` 全量通过
- [x] 3.3 单测覆盖 preset partial 状态计算
- [x] 3.4 单测覆盖搜索过滤命中分组与技能场景

## Verification Checklist

- [x] `openspec validate update-skills-presets-multi-select --strict --no-interactive`
- [x] `just qa`

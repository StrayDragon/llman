---
depends_on: []
rules_edit_acked: true
branch: sdd/remove-sdd-skill-set-meta
base_sha: 319ec051fa9da164a7ebfd33de915af3a4c11777
checkpointed: true
checkpoint_sha: 319ec051fa9da164a7ebfd33de915af3a4c11777
---

# 退役托管 skill 的 skill_set 元数据与 llman_sdd 块

## Why

`skill_set` 与刚退役的 `bdd_mode` 同类：没有任何代码分支读取安装产物的
`skill_set`——r90 候选集清理读 config（DEFAULT_SKILL_FILES + extra_skills），
optional 条件渲染用 `extra_skill_*` 变量，`llman-sdd-` 前缀才是清理边界。
它是纯装饰元数据，每 skill 每次渲染白付一行 + 一段枚举门禁机器
（validate_llman_sdd_meta + 专用 i18n 文案 + 双路调用）。

用户决策：整体移除 `metadata.llman_sdd` 块（frontmatter 只剩
`metadata.version`，供 check-skills-version.py 消费）。

## What Changes

- 36 个模板（双 locale）删除 `metadata.llman_sdd` 子块（含 `skill_set` 行）；
  渲染变量 `skill_set` 与 `load_skill_template` 的 skill_set 参数退役。
- `skill_consistency.rs` 瘦身为「安装产物卫生检查」：仅保留 unrendered
  MiniJinja 残留检查（`{% ... %}`），llman_sdd/skill_set 解析与校验删除。
- r95 整条删除（锁定规则，ack 已带）：其主体（llman_sdd/skill_set 门禁）
  不复存在；连带删除其 2 个 @executable 场景（场景 1 的行为随门禁消失，
  场景 2 前提不复成立）。unrendered-jinja 检查无合约条目，为实现层行为。
- i18n `skill_set_invalid` 键删除。
- 测试侧：skill_consistency 单测重写为 jinja-only；bdd_steps 的
  given_skill_dir/global_skills_config、it 的 markdown-override fixture
  同步去除 metadata 写入。

## Capabilities

- `sdd-workflow`：r95 删除 + 2 场景删除（锁定规则，ack）。

## Impact

- 受影响范围：36 处模板 frontmatter、渲染参数链、skill_consistency、
  渲染产物 resync、bdd_steps/it 测试、1 条规则 + 2 场景删除、1 个 i18n 键。
- 行为变化：托管 skill frontmatter 只剩 name/description/metadata.version；
  `llman-sdd-` 前缀（r90）继续作为托管边界唯一标识。
- 不做：`metadata.version` 改动（check-skills-version.py 消费方保留）；
  jinja 残留检查的合约化（如需另开 change）。
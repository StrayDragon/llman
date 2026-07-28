---
depends_on: []
branch: sdd/promote-draft-skill
base_sha: ca4d48560baaf03b8f771d4810ef2c9c564e37aa
checkpointed: true
checkpoint_sha: ca4d48560baaf03b8f771d4810ef2c9c564e37aa
---

## Why

当前「快速记一个提案（draft）」的能力被埋住了：它既作为内联段存在于默认 `llman-sdd-propose` 技能（`templates/sdd/{en,zh-Hans}/skills/llman-sdd-propose.md` 的「轻量 draft 路径」段），又以可选技能 `llman-sdd-new-change` 的形式存在（`OPTIONAL_SKILL_FILES` 首项，默认不安装）。结果是：

- `propose` 技能单文件承载两个意图（完整 propose + 快速 draft），阅读认知负担重——用户反馈想「分离」。
- 现成的 `llman-sdd-new-change` 是 optional，默认不安装、基本无人用，且与 propose 内联段语义重复。
- 实际用户场景（先把 idea / 未来需求记下来）非常高频，但缺少一个默认可用、命名直白、职责单一的入口。

## What Changes

1. **重命名 + 提升**：把可选 `llman-sdd-new-change` 重命名为 `llman-sdd-draft`，并从 `OPTIONAL_SKILL_FILES` 移到 `DEFAULT_SKILL_FILES`（成为默认安装的技能）。
2. **重写 draft 技能模板**：职责单一化为「仅 draft proposal（draft shell，不强制 tasks/design/specs/attach）」，复用现有 `change new --from` CLI 原语。
3. **裁剪 propose 内联 draft 段**：`llman-sdd-propose` 模板移除完整 draft 路径描述，替换为一句指向 `llman-sdd-draft` 的引导（「仅记草案用 llman-sdd-draft」）。
4. **同步 Rust 常量与 embed 表**：`DEFAULT_SKILL_FILES` / `OPTIONAL_SKILL_FILES` / `OPTIONAL_SKILL_NAMES` / `skill_description()` / `embedded_template` match arm 全部从 `new-change` 改为 `draft`。
5. **向后兼容自愈说明**：已 init 项目 config 里残留的 `extra_skills: [llman-sdd-new-change]` 在下次 `update-skills` 时会被自动忽略（不匹配 optional 列表），旧 `llman-sdd-new-change` 目录会被 `cleanup_stale_skills` 自动清理——无需显式迁移代码。

## Capabilities

- `sdd-structured-skill-prompts`（扩 r117：独立 draft 技能的默认安装与职责单一化）
- `skills-management`（间接：技能列表变化，但无新行为合约）

## Impact

- **用户可见**：`llman sdd init` / `update-skills` 后默认多出 `llman-sdd-draft` 技能；propose 技能变薄。
- **已 init 项目**：下次 `update-skills` 自动迁移（删旧 new-change 目录、装新 draft）；config 里失效的 extra_skills 条目被静默忽略。
- **测试**：`update_skills_does_not_write_optional_skills_by_default` 等断言 new-change 不默认写入的测试需更新为断言 draft 默认写入。
- **AGENTS.md / 文档**：可选增强能力表中的 skill 列表需同步（new-change → draft）。

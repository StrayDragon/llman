---
id: add-meta-skill-dynamic-prompts
depends_on:
  - update-skill-bdd-mode-conditioning
stage: draft
---

# Proposal (DRAFT): 元 Skill + `llman sdd` 动态提示词

> **状态：draft only — 不在本次实现。** 待 `update-skill-bdd-mode-conditioning`
> 应急方案落地并观察后，再评估是否升级为本 change 的 full propose。

## Why

应急方案（条件渲染静态 SKILL.md）仍有局限：

1. Cursor 等宿主把整份 SKILL.md 注入上下文，即使已按 BDD 裁剪，pipeline 全貌 + 命令表仍偏长。
2. `extra_skills` / change stage / 当前分支绑定等**运行时状态**无法进入静态文件，agent 仍可能推荐未安装 skill。
3. 双模式 / 多 optional 组合使模板 `{% if %}` 分支继续膨胀。

目标方向：项目内只安装一个 **bootstrap 元 skill**；执行时由 `llman sdd` 根据
config + change stage + extra_skills 输出当步指令（动态提示词），从根上解决主动条件动态指示。

## What Changes（意向，未承诺）

1. 新增 CLI：例如 `llman sdd agent prompt --skill <phase> --change <id>`（名称待定），
   stdout 为当步完整指示（或 JSON sections）。
2. 默认只生成/保留一个元 skill（如 `llman-sdd`），description 指引 agent 先调上述命令。
3. 与已落地的 `metadata.llman_sdd.*` 门禁兼容；静态全量 skills 可变为可选 `extra_skills` 或废弃。
4. 评估宿主限制：Cursor skill 是否允许「短 skill + 工具拉取正文」工作流。

## Non-goals（draft 阶段）

- 不改当前默认 skill 集合安装行为（由应急 change 负责）。
- 不删除 mermaid；动态提示词仍可内嵌当步图。

## Open Questions

- 动态提示词缓存与版本戳（避免 stale 指示）？
- 离线 / 无 llman 二进制时的降级？
- 与 ACP / eval 管线如何对齐？

## Next

评估会议后再决定：升级本 draft → full propose，或关闭并保留 docs 记载。

## Why

当前 llman sdd 仅生成简短提示块且缺少 skills 输出与 locale 支持，导致 AI 无法稳定发现 llman sdd 工作流，也无法按用户语言生成上下文与技能内容。我们需要用更接近 OpenSpec 1.0 的方式提供完整方法论注入、可维护的 skills 生成能力，以及项目级 locale 配置。

## What Changes

- 新增 `llmanspec/config.yaml`，记录 `version`/`locale` 与 skills 默认路径配置
- sdd 模板改为 locale-aware 加载，提供 en / zh-Hans 模板并支持回退链
- `llman sdd init/update` 生成或刷新 root `AGENTS.md` stub，并将 `llmanspec/AGENTS.md` 升级为完整方法论模板
- 新增 `llman sdd update-skills`，生成/更新 Claude Code 与 Codex 的 skills（支持 `--all` 与 `--tool`，不生成 slash commands）
- 增加模板区域引用解析（基于 `region` 块复用文档片段，避免重复维护）
- sdd validate 增强可行动的修复提示（缺段落、缺描述、场景格式错误、无 delta 等）
- 补齐 `locales/app.yml` 的 `sdd.*` 英文词条，消除 key 回退

## Capabilities

### Modified Capabilities
- `sdd-workflow`: locale/config、AGENTS stub、skills 生成、validate 提示增强

## Impact

- 受影响规范：`sdd-workflow`
- 受影响代码：`src/sdd/*`, `src/cli.rs`, `templates/sdd/**`, `locales/app.yml`

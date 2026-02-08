## Context

llman sdd 当前仅提供基础 skills（onboard/new-change/show/validate/archive），缺少 OpenSpec 风格的完整流程技能。现有用户已依赖 `llman-sdd-*` 命名与 `llman sdd update-skills` 生成机制，因此升级需要在不破坏现有行为的前提下扩展能力。

本变更目标是在 SDD 范围内补齐流程技能，并加入可复用 prompt 注入能力，使 skill 内容更稳定、更可维护。

## Goals / Non-Goals

**Goals:**
- 新增工作流 skills：`llman-sdd-explore`、`llman-sdd-continue`、`llman-sdd-apply`、`llman-sdd-ff`、`llman-sdd-verify`、`llman-sdd-sync`
- 支持 `{{prompt: <name>}}` 注入语法，减少 skills 中重复文本
- 新增 `llmanspec/config.yaml` 的 `prompts.custom_path` 可选配置
- 保持 `llman-sdd-*` 前缀和现有命令兼容

**Non-Goals:**
- 不直接依赖 `openspec` CLI
- 不改动 `llman skills`（skills-management）能力
- 不将 prompts 注入 `AGENTS.md`
- 不实现自动化 delta specs 合并引擎

## Decisions

### Decision 1: Skills 模板扩展策略

采用“模板翻译 + 命令映射”策略：基于 OpenSpec 同类技能结构，改写为 llman sdd 语义，并统一使用 `llman-sdd-*` 命名。

新增技能：
- `llman-sdd-explore`
- `llman-sdd-continue`
- `llman-sdd-apply`
- `llman-sdd-ff`
- `llman-sdd-verify`
- `llman-sdd-sync`

保留现有技能：
- `llman-sdd-onboard`
- `llman-sdd-new-change`
- `llman-sdd-show`
- `llman-sdd-validate`
- `llman-sdd-archive`

### Decision 2: Prompt 注入机制

在模板系统中新增 `{{prompt: <name>}}` 语法，注入 `prompts/<name>.md` 内容。

加载优先级（高 → 低）：
1. `llmanspec/config.yaml` 中 `prompts.custom_path`
2. 项目级 `templates/sdd/<locale>/prompts/`
3. 内置模板 `templates/sdd/<locale>/prompts/`

其中 locale 回退链保持一致：`zh-Hans` → `zh` → `en`。

### Decision 3: 配置模型扩展

在 `llmanspec/config.yaml` 新增：

```yaml
version: 1
locale: en
skills:
  claude_path: ".claude/skills"
  codex_path: ".codex/skills"
prompts:
  custom_path: null
```

`custom_path` 为可选字段；未配置时退回项目级/内置来源。

### Decision 4: llman-sdd-sync 范围

`llman-sdd-sync` 在 V1 定义为“可复现的人工作业协议”技能，而非自动合并器。它必须提供明确步骤：
1. 检查 change 的 delta specs
2. 指导人工将 ADDED/MODIFIED/REMOVED/RENAMED 变更同步到主 specs
3. 运行 `llman sdd validate --specs` 验证结果

## Risks / Trade-offs

### Risk 1: 模板同步成本
OpenSpec 上游模板变化可能导致翻译模板滞后。

**Mitigation:** 维持模板版本号与来源映射，更新时做差异审查。

### Risk 2: Prompt 覆盖来源冲突
同名 prompt 在 custom/project/embedded 多处存在时，行为依赖优先级。

**Mitigation:** 在 spec 中固定优先级并添加冲突测试。

### Risk 3: 范围膨胀
若在本变更中同时做 AGENTS 注入或自动 sync，实施与测试复杂度会显著增加。

**Mitigation:** 明确列为 Non-Goals，后续独立提案处理。

## Migration Plan

1. 新增 prompts 模板目录与文件（en/zh-Hans 对齐）
2. 扩展模板渲染管道支持 `{{prompt: ...}}`
3. 扩展 `SddConfig` 与 schema（`prompts.custom_path`）
4. 新增 6 个 workflow skills 模板并纳入 `update-skills`
5. 补齐验证与文档

## Open Questions

无阻塞开放问题。

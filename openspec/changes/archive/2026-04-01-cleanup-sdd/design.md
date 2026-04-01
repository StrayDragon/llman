## Context

当前仓库中 SDD 体系同时存在：

- new track：以 canonical table/object ISON 为核心（`templates/sdd/**` + `llman sdd ...`）
- legacy track：以 `llman sdd-legacy ...` 命令组与 `templates/sdd-legacy/**` 模板为核心，并包含 legacy JSON-in-` ```ison ` 解析/校验语义与若干 legacy-only 交互/提示

我们已经完成向新技术栈的验证与迁移，因此继续保留 legacy track 的主要代价是：

- 双轨模板与提示词内容漂移的维护成本
- CLI/校验语义分叉导致的复杂度与测试负担
- 对用户与贡献者的路径选择困惑

本变更目标是将仓库彻底收敛到单轨 new-style SDD。

## Goals / Non-Goals

**Goals:**
- 移除 `llman sdd-legacy ...` 命令与所有对应实现（包含 TemplateStyle/legacy parser/legacy-only 子命令）。
- 删除 `templates/sdd-legacy/**`，并移除任何渲染/生成的 legacy 分支逻辑。
- 更新所有 specs/docs/tests，确保仓库内不再出现 `sdd-legacy` 风格与命令提示。
- 引入并接入 `llman-sdd-propose`（从 `openspec-propose` 迁移适配），作为 llman SDD 的“快速创建变更并生成工件”的入口之一。
- `llman x sdd-eval` 等实验评测仅保留 new-style 路径（移除 `sdd-legacy` style 与 variants）。

**Non-Goals:**
- 不提供自动迁移 legacy JSON payload 的命令或一键转换工具（假设迁移已完成）。
- 不维持 legacy 行为兼容或旧的提示/输出文本稳定性。
- 不保留 “legacy 仍可用” 的软连接/别名命令（例如 `llman sdd-legacy` 的隐藏入口）。

## Decisions

1. **硬删除 legacy track（无兼容保留）**
   - 删除 `templates/sdd-legacy/**`。
   - 删除 `llman sdd-legacy` 子命令组与相关代码/测试。
   - 移除 `TemplateStyle::Legacy`、`WorkflowStyle::SddLegacy` 等枚举分支与任何 `sdd-legacy` 相关配置/解析。

2. **legacy payload 的错误提示仅给出“重写为 canonical ISON”指导**
   - 当检测到 legacy JSON-in-` ```ison ` 的 payload 时，不再提示 “改用 `llman sdd-legacy ...`”。
   - 错误信息应给出最短可操作 guidance：指出 canonical blocks（`object.spec` / `table.requirements` / `table.scenarios` 等）与示例参考位置（例如模板 unit `spec/ison-contract`）。

3. **SDD eval DSL 收敛为单 style**
   - `sdd-eval` 的 DSL/playbook 配置去掉 `style: sdd-legacy`，只保留 `style: sdd`（或等价的单一默认值）。
   - 删除 legacy variants（例如 `sdd-legacy-codex`）并相应简化执行逻辑与报表结构。

4. **`llman-sdd-propose` 作为语义别名引入**
   - 迁移 `openspec-propose` 的“创建变更 + 一次性生成 proposal/specs/design/tasks”的语义，适配到 `llmanspec/` 与 `llman sdd` 命令集。
   - 实现上可复用现有 `llman-sdd-ff` / `llman-sdd-new-change` 的表达与 unit 注入结构，避免重复维护。

5. **Spec 清理策略**
   - 删除/退役描述 legacy 机制的 capability（`sdd-legacy-compat`）。
   - 其他涉及 legacy 的 specs（`sdd-workflow`、`sdd-ison-pipeline`、`sdd-structured-skill-prompts`、`sdd-eval-*`）改为仅描述单轨 new-style 行为。

## Risks / Trade-offs

- **[BREAKING] 仍依赖 legacy 仓库/内容的用户将无法继续使用旧语义** → 缓解：本变更假设已完成迁移；同时提供明确的 canonical ISON 重写提示与文档指引。
- **删除 legacy 分支可能暴露隐藏耦合（eval/playbook、模板注入、校验路径）** → 缓解：逐步拆分提交任务、补齐/更新集成测试覆盖，并以 `just check` 为质量门禁。

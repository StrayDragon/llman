## Context

OpenSpec 已将 OPSX（`/opsx:*`）作为默认工作流：以“动作（actions）而非阶段（phases）”驱动变更创建、继续、实施与归档。OPSX 的优势在于：

- 入口清晰（`/opsx:new`、`/opsx:continue`、`/opsx:apply`…）
- 交互自然（可以随时回到 proposal/specs/design/tasks 修正）
- 生态成熟（不同工具通过 commands 适配，skills 作为通用执行能力）

llman sdd 当前在 `llmanspec/` 内提供 spec-driven 的文件结构与基础 skills 生成，但：
- 不提供 OPSX 的 slash commands 绑定（工具入口缺失）
- 现有升级方案倾向引入 prompt 注入语法（`{{prompt: ...}}`），会带来额外解析/优先级/测试维护负担

因此，本变更选择更简单且更贴近上游的方式：**直接 vendor OpenSpec OPSX 的命令与 skills 模板到 llman 的 `templates/sdd/`**，并通过 `llman sdd update-skills` 生成“OPSX 命令 + workflow skills”。

## Goals / Non-Goals

**Goals:**
- 在 llman sdd 中提供 OPSX 风格入口：为 Claude Code 与 Codex 安装 **OPSX slash commands**（仅保留 `/opsx:*` 这一套）
- 在 llman sdd 中提供与 OPSX 对齐的 workflow skills（覆盖 explore/onboard/new/continue/ff/apply/verify/sync/archive/bulk-archive）
- 实现方式以“复制并调整模板”为主：把上游 OpenSpec OPSX 模板落入 `templates/sdd/`，保持可读、可 diff、可维护

**Non-Goals:**
- 不直接依赖 `openspec` CLI
- 不改动 `llman skills`（skills-management）能力
- 不实现 schema-driven 工作流引擎（`status` / `instructions` / schema.yaml 等能力属于更大范围）
- 不实现自动化 delta specs 合并引擎（`sync` 仍以“可复现的人工作业协议”为主）

## Decisions

### Decision 1: 以“vendor 模板”替代“prompt 注入”

不新增 `{{prompt: ...}}` 之类的模板占位符语法与多层加载优先级。

取而代之：
- 直接从上游 OpenSpec repo 复制 OPSX 相关模板到 `templates/sdd/<locale>/`（vendor by copy）
- 在 llman repo 内完成必要的“路径/命令/术语”调整（`openspec/` → `llmanspec/`、`openspec` CLI → `llman sdd`）
- 保留现有 `{{region: ...}}` 机制作为少量共享片段能力（不新增新语法）

### Decision 2: 仅保留新的 OPSX commands 集合

不再引入旧式 legacy commands（如 `/openspec:proposal` 体系）。llman sdd 生成/刷新命令绑定时仅覆盖：

- `/opsx:explore`
- `/opsx:onboard`
- `/opsx:new`
- `/opsx:continue`
- `/opsx:ff`
- `/opsx:apply`
- `/opsx:verify`
- `/opsx:sync`
- `/opsx:archive`
- `/opsx:bulk-archive`

工具适配目录（V1 仅覆盖 llman 当前支持的 Claude Code 与 Codex）：
- Claude Code：`.claude/commands/opsx/`
- Codex：`.codex/prompts/`（项目级）

### Decision 3: skills 与 commands 的映射关系（llman 侧）

OPSX commands 的动词集合与 llman sdd skills 一一对应（便于用户用 `/opsx:*` 驱动工作流，同时在“无 slash 支持的场景”仍可直接使用 skills）：

- `/opsx:explore` → `llman-sdd-explore`
- `/opsx:onboard` → `llman-sdd-onboard`
- `/opsx:new` → `llman-sdd-new-change`
- `/opsx:continue` → `llman-sdd-continue`
- `/opsx:ff` → `llman-sdd-ff`
- `/opsx:apply` → `llman-sdd-apply`
- `/opsx:verify` → `llman-sdd-verify`
- `/opsx:sync` → `llman-sdd-sync`
- `/opsx:archive` → `llman-sdd-archive`
- `/opsx:bulk-archive` → `llman-sdd-bulk-archive`

（`llman-sdd-show` / `llman-sdd-validate` 保持作为辅助技能，但不绑定到 OPSX commands。）

### Decision 4: sync 的范围保持“人工作业协议”

`llman-sdd-sync` 在 V1 仍定义为“可复现的人工作业协议”（不引入自动 delta 合并引擎）。它需要明确、可重复的步骤，并以 `llman sdd validate --specs` 作为验证闭环。

## Risks / Trade-offs

### Risk 1: 上游模板演进导致 vendor 滞后
OpenSpec OPSX 模板可能持续迭代，llman vendored 模板可能落后。

**Mitigation:** 在 vendored 模板头部记录来源（OpenSpec 路径 + 版本/日期），并通过 `just check-sdd-templates` 保持 locale parity；后续如需自动化同步，再单独提案。

### Risk 2: 命令/技能双份内容的维护成本
同一动作既有 command 文件又有 skill 文件，存在内容漂移风险。

**Mitigation:** 尽量以同一上游模板为源（复制后一次性调整），并在本 repo 内维持 1:1 的动作集合与版本号；必要时引入轻量的生成脚本（但不在本变更强行引入新模板语法）。

### Risk 3: 范围膨胀（schema engine）
若本变更同时实现 `status` / `instructions` / schema engine，将明显扩大实现与测试面。

**Mitigation:** 明确列为 Non-Goals，后续用独立 change 推进（现有 `upgrade-mixin-opsx-to-llman-sdd` 可作为候选承载）。

## Migration Plan

1. Vendor OpenSpec OPSX templates 到 `templates/sdd/<locale>/` 并完成 llman 侧术语/路径调整
2. 扩展 `llman sdd update-skills`：生成/刷新 workflow skills，并额外写入 opsx commands 绑定目录
3. 更新 `templates/sdd/*/agents.md`：允许并引导使用 `/opsx:*`（移除“不要添加 slash commands”的限制语句）
4. 添加最小测试覆盖（至少验证生成路径、动作集合、locale 回退）

## Resolved Decisions

1. Codex 的 OPSX commands 仅安装到项目级 `.codex/prompts/`；不支持写入 user-global（如 `$CODEX_HOME/prompts`）。如需用户级别配置，由用户自行使用 `llman skills` 等现有机制管理。
2. V1 需要提供 legacy commands 的迁移：当检测到 legacy 命令绑定（例如 `.claude/commands/openspec/`、`.codex/prompts/openspec-*.md`）时，`llman sdd update-skills` 在交互模式下 MUST 进行二次确认（推荐第二次为“输入确认词”），确认后删除 legacy 并生成 OPSX commands；在 `--no-interactive` 模式下 MUST 停止并提示用户改用交互模式完成迁移。

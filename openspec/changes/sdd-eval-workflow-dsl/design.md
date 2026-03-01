## Context

当前 `llman x sdd-eval` 的 playbook 模型（`src/x/sdd_eval/playbook.rs`）更像“固定流水线配置”，runner 在 `src/x/sdd_eval/run.rs` 内以固定顺序执行：

- 为每个 variant 创建 run/workspace/logs/artifacts 目录
- 复制项目到 workspace 并初始化 SDD（new vs legacy）
- 启动 ACP agent 并执行固定迭代次数的 SDD loop
- 落盘 per-variant 指标并生成报告

这种结构在 v1 足够落地，但对后续实验扩展不友好：任何“在评测流程里插入/重排通用步骤”的需求（比如依赖安装、单独的测试/收集、额外 artifact 打包、不同的 pre/post 处理）都要改代码；同时 YAML 缺少 schema，写错字段/层级成本高。

本变更将 playbook 重构为 GitHub Actions 风格的工作流 DSL（jobs/steps + matrix variants + 内置 actions），并增加自动生成的 JSON schema 用于 YAML LSP。

关键约束：
- `sdd-eval` 是实验命令：允许 BREAKING 变更（不做旧格式兼容/迁移）。
- 安全边界必须保持：preset secrets 不落盘、日志与 artifacts 需 redaction、workspace sandbox 严格（禁止越界读写/命令）。
- 测试不得触碰真实用户配置：必须在 `TempDir` + `LLMAN_CONFIG_DIR` 隔离环境下运行。

## Goals / Non-Goals

**Goals:**
- 用 workflow/jobs/steps 表达评测流程：可组合、可扩展、能表达“通用剧本 + AB 组分别执行”。
- 提供内置 actions，覆盖当前固定 pipeline 的核心能力，避免 playbook 直接绑定实现细节。
- 支持 `run:` 步骤（默认可用）用于通用脚本化，但仍遵守 sandbox + allowlist 安全模型。
- 通过 `schemars` 自动生成 playbook JSON schema，并在 `init` 模板里写入 `# yaml-language-server: $schema=...` 头注释。
- 默认串行执行（matrix variant 依次跑），确保可复现与易 debug。

**Non-Goals:**
- 不实现完整 GitHub Actions 语义（不做 `if:` 表达式语言、`outputs`、复杂 `strategy`、环境矩阵笛卡尔积等）。
- 不支持旧 playbook schema（`version: 1` 及其字段）继续运行，也不提供自动迁移命令。
- 不引入“任意 shell 脚本”执行：`run:` 仅允许单命令调用（无 `&&`/管道/重定向语义）。
- 不扩展 ACP 能力面；仍以满足 Claude Code / Codex ACP 的最小集为边界。
- 不引入并发开关（例如 `--jobs`），也不提供 per-step `env`/`timeout`/`continue-on-error` 语义。
- 不抽象为更通用的 `llman x workflow`（仅在 sdd-eval 内部落地）。

## Decisions

### Decision: New playbook top-level model (no version field)
新 playbook 不包含 `version`，直接替换旧格式，字段以强类型模型定义（`serde(deny_unknown_fields)`）并在验证阶段给出明确错误。

顶层键（requiredness 明确）：
- `task`（必填：title/prompt）
- `variants`（必填：map；key 为 `variant_id`）
- `workflow.jobs`（必填）
- `sdd_loop`（可选：用于为内置 action 提供默认 loop 配置；若缺失则使用默认值）
- `report`（可选：ai_judge/human）
- `name`（可选）

并增加显式的 legacy 检测：若 playbook 顶层出现 `version` 字段（例如 `version: 1`），必须以可操作的错误提示用户更新到新 DSL。

### Decision: Run directory layout is created up-front
`llman x sdd-eval run` MUST always create the run directory and base layout before executing any workflow steps:
- copy the playbook into `<run_dir>/playbook.yaml`
- write `<run_dir>/manifest.json`
- for each defined variant id, create:
  - `<run_dir>/variants/<variant_id>/workspace/`
  - `<run_dir>/variants/<variant_id>/logs/`
  - `<run_dir>/variants/<variant_id>/artifacts/`

This ensures:
- matrix job sandbox roots always exist before the first step runs
- built-in actions can be composed without needing implicit “magic” directory creation

### Decision: Variants as a map + matrix expansion
`variants` 由 list 改为 map（key 为 `variant_id`），并通过 `workflow.jobs.<job>.strategy.matrix.variant` 展开执行。

约束：
- `variant_id` 作为磁盘路径片段，必须满足安全正则 `^[a-zA-Z][a-zA-Z0-9_-]*$`。
- matrix 中引用的 `variant_id` 必须存在，否则 fail-fast。

### Decision: Workflow engine is deterministic and serial by default
实现一个轻量 workflow runner：
- `needs` 构建 job DAG；未知 job / 依赖环直接报错。
- Job 运行顺序：拓扑序 + YAML 顺序作为 tie-breaker（稳定、可复现）。
- matrix variant 展开默认串行执行（本变更不引入任何并发开关/参数）。

实现约束（避免实现者自行猜测）：
- 必须保留 `workflow.jobs` 的 YAML 声明顺序，否则无法满足 “YAML 顺序 tie-breaker” 的要求。
  - Rust 实现建议：将 `workflow.jobs` 反序列化到 `indexmap::IndexMap`（或等价“保持插入顺序”的 map），避免 `BTreeMap`（按 key 排序）与 `HashMap`（顺序不稳定）。

### Decision: Steps are either `uses` (built-in actions) or `run` (allowlisted)
每个 step 只能二选一：
- `uses`: 调用内置 action（强类型 `with` 输入；便于 schema 补全与校验）
- `run`: 执行单条本地命令（用于通用脚本化）

`run` 的安全语义：
- `run` 解析为 argv（仅做引号分词，不通过 shell；不支持操作符语义）
- 以 argv[0] 做 allowlist 校验（复用 ACP terminal allowlist 的策略）
- cwd 默认是预置 sandbox 根，且仅允许通过相对路径进入该根目录下的子目录：
  - matrix job：variant workspace
  - 非 matrix job：run_dir
- `run` step MUST NOT 注入 preset env（preset env 仅用于启动 ACP agent 子进程），避免把 secrets 引入通用命令执行路径。
- 记录输出时做 secret redaction（复用 `SecretSet`），并做大小截断（避免 artifacts 膨胀）。
- `run` step 的执行记录会计入与 ACP terminal commands 相同的指标类别，以保证 `report` 的 terminal summary 覆盖 workflow 的 `run` steps。

### Decision: Built-in actions cover current pipeline primitives
定义 action 注册表（`builtin:sdd-eval/...`），首批 actions 覆盖现有能力：
- `workspace.prepare`: 将项目复制进 workspace（带 skip 规则），并确保 logs/artifacts 目录存在
- `sdd.prepare`: 根据 `variant.style` 执行 new/legacy 的 SDD init/update
- `acp.sdd-loop`: 注入 preset env 启动 ACP agent，执行 bounded loop，落盘 session log 与 metrics
- `report.generate`: 聚合 metrics，生成 report（可选 AI judge + human pack）

这些 actions 的 `with` 结构使用 Rust struct 建模并 derive `JsonSchema`，以便 schema 可补全。

### Decision: Schema generation integrates with existing `llman self schema` workflow
沿用现有 schema 生成与校验体系：
- 在 `artifacts/schema/` 下新增 playbook schema 输出目录（例如 `artifacts/schema/playbooks/en/`）
- 扩展 `llman self schema generate` 生成新 playbook schema 文件
- 扩展 `llman self schema check` 校验 playbook schema（使用内置模板实例作为样例，避免依赖用户文件）
- `llman x sdd-eval init` 模板文件顶部写入 `# yaml-language-server: $schema=...` 指向对应 raw URL

## Risks / Trade-offs

- **`run:` 不支持完整 shell 语义** → 牺牲灵活性换取可控与可复现；通过清晰文档与错误提示降低误解成本。
- **command allowlist 过严** → 可能挡住部分真实项目测试命令；本变更的 allowlist 不可在 playbook 中配置，扩展 allowlist 需要代码变更。
- **BREAKING playbook** → 实验期可接受；必须在 `--help` 与错误中明确，并提供新模板作为迁移参考。
- **schema 维护成本** → 通过 `schemars` 自动生成降低成本；同时确保 `check` 覆盖避免 drift。

## Migration Plan

无自动迁移：旧 playbook（`version: 1`）将被拒绝。
维护者通过 `llman x sdd-eval init` 生成新模板并按需迁移字段与步骤。

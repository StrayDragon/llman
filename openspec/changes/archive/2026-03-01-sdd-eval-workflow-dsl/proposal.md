## Why

当前 `llman x sdd-eval` 的 YAML playbook 更接近“固定流水线配置”：runner 在实现里硬编码了 workspace 初始化、SDD 模板初始化、ACP 执行、指标落盘与报告生成等步骤。对维护者来说，这带来三个直接问题：

1) **难以复用与组合**：一旦希望在评测前后插入通用步骤（例如依赖安装、lint/test、基准数据准备、额外的收集与打包），往往需要改代码而不是改剧本。
2) **AB 组扩展成本高**：当前 variants 仅覆盖少数固定字段（style/agent），当实验维度增加（比如更多步骤、不同的执行顺序、不同的收集策略）时，DSL 难以承载。
3) **编写体验差**：YAML 结构缺少可用的 schema 支持，容易写错字段名/层级；同时也难以在编辑器中获得补全与诊断。

我们希望将 sdd-eval 的 playbook 升级为一种更通用、可组合、可扩展的“工作流 DSL”，并且提供自动生成的 JSON schema 来支撑 YAML LSP（补全/提示/校验），以降低实验脚本的维护成本并为未来更通用的实验 runner 打基础。

## What Changes

- **BREAKING**：替换 `llman x sdd-eval` playbook YAML 结构为 GitHub Actions 风格的工作流 DSL（不兼容旧写法；实验期直接替换，不做迁移与兼容）。
  - 新 DSL 引入 `workflow.jobs.<job_id>.steps[]`（`uses:` 内置 actions + `run:` 命令步骤）、`needs` 依赖与 `strategy.matrix.variant`（对 AB 组展开执行）。
  - `variants` 从 list 改为 map（以稳定的 `variant_id` 作为 key），便于 matrix 引用与路径命名。
- 内置一组可复用的 actions（类似 GitHub Actions 的 `uses:`），用于覆盖当前 sdd-eval 的核心流程（workspace 准备、SDD 初始化、ACP SDD loop、报告生成）。
- 允许在 workflow 中编写通用 `run:` 步骤（用于测试、收集、脚本化处理），但仍必须遵守 sandbox/allowlist 安全模型（限制 cwd/路径与允许的命令集合），避免将 playbook 变为任意命令执行入口。
- 为新 playbook 模型提供**自动结构生成**的 JSON schema（基于 Rust `schemars`），并：
  - 将 schema 输出到仓库 `artifacts/schema/**`（可被发布到 `raw.githubusercontent.com`）；
  - `llman x sdd-eval init` 生成的 YAML 顶部写入 `# yaml-language-server: $schema=...` 注释，启用 YAML LSP 补全与诊断。

## Capabilities

### New Capabilities
- `sdd-eval-workflow-dsl`: 为 `llman x sdd-eval` 定义 GitHub Actions 风格的工作流 DSL（jobs/steps、内置 actions、matrix=variants、轻量插值上下文），并定义默认串行执行与可复现的执行语义。

### Modified Capabilities
- `sdd-eval-acp-pipeline`: 更新 playbook 解析与执行模型：从固定 pipeline 配置切换为 workflow 驱动；在不削弱 secrets redaction 与 workspace sandbox 的前提下，支持 actions + `run:` steps 的可组合执行。
- `config-schemas`: 扩展 schema 生成范围，新增 `sdd-eval` playbook 的 JSON schema 输出与校验（并保持既有 config schemas 行为与路径不回归）。

## Impact

- **CLI / UX**
  - `llman x sdd-eval init` 生成的模板结构变更（包含 schema header + workflow 示例）。
  - 旧 playbook 不再可用（实验期 BREAKING 行为在帮助与错误信息中必须明确）。
- **代码结构**
  - `src/x/sdd_eval/playbook.rs`：替换为新 DSL 模型与校验逻辑。
  - `src/x/sdd_eval/run.rs`：执行路径由“固定顺序”改为 workflow runner（jobs/steps/matrix）。
  - 新增内置 actions 注册表与实现模块（复用当前固定 pipeline 的能力）。
  - 复用/抽取 terminal allowlist/sandbox 逻辑以服务 `run:` step，同时保持 ACP runner 的安全边界不变。
- **Schema / 工程化**
  - `artifacts/schema/**` 新增 playbook schema 文件；`llman self schema generate/check` 覆盖该 schema 的生成与校验。
- **测试**
  - 更新 `tests/sdd_eval_tests.rs` 以使用新 DSL；保留并强化对 sandbox 与 secrets redaction 的回归测试。

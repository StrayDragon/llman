## Why

目前我们有两套 Spec-Driven Development（SDD）工作流：`sdd`（new）与 `sdd-legacy`（legacy）。当团队希望评测“同一任务在不同工作流与不同 coding agent（Claude Code / Codex 等）下的产出差异”时，需要大量手工步骤（init、配置 agent、重复投喂任务、跑循环、收集 diff/测试结果、汇总打分）。这些步骤难以复现、难以规模化、难以对比，也很难沉淀为可持续迭代的评测基线。

同时，ACP（Agent Client Protocol）正在成为 Claude Code / Codex 等 coding agent 的标准接入方式。我们希望 `llman` 能作为“评测驱动器/runner”，以统一方式启动 ACP agent 并提供最小 editor 能力，从而把评测流程从“手工试跑”升级为“可脚本化的流水线”。

另外，评测流水线在启动 ACP agent 时需要鉴权信息（API key 等）。我们已经在 `llman x cc`（亦即 `llman x claude-code`）与 `llman x codex` 下建立了 account/group 配置体系；评测流水线必须复用该体系，并保证 secrets 不会在 playbook、stdout/stderr、或 run artifacts 中泄漏。

## What Changes

- 新增实验性命令 `llman x sdd-eval`：使用 playbook（YAML 剧本）驱动一次或多次 SDD 评测 run。
  - `init`：生成可编辑的 playbook 模板。
  - `run`：执行 playbook，按 variants（workflow style × agent）创建隔离 workspace，并以固定迭代次数驱动 SDD 循环。
  - `report`：汇总客观指标，生成报告；支持（可选）AI Judge；生成可离线人工打分包。
  - `import-human`：导入外部人工打分结果并合并到报告。
- 以项目内临时目录管理 playbook 与 run artifacts：
  - playbook：`<project>/.llman/sdd-eval/playbooks/`
  - runs：`<project>/.llman/sdd-eval/runs/<run_id>/`
- 新增 ACP runner（client 侧）：`llman` 作为 ACP client，启动并驱动 `claude-code-acp` / `codex-acp` 等 ACP agent 进程，提供最小文件读写与 terminal 能力（用于 repo 修改与验证）。
- 新增“ACP 启动预设”能力：playbook 可引用 `llman x cc` / `llman x codex` 的 account group，在运行时注入 env 启动 ACP agent；实现必须保证敏感值不被打印或落盘（仅允许记录 group 名称与 env 键名，且默认不记录值）。

## Capabilities

### New Capabilities
- `sdd-eval-acp-pipeline`: 提供 `llman x sdd-eval` 的评测流水线（playbook DSL + variant runner + report + AI/human scoring export/import），并通过 ACP 驱动 Claude Code / Codex 等 coding agent。

### Modified Capabilities

<!-- none -->

## Impact

- CLI
  - 新增实验性入口 `llman x sdd-eval` 与多个子命令；需要明确 help、错误输出、非交互行为与退出码策略。
- 落盘与文件结构
  - 在目标项目根目录新增/使用 `.llman/sdd-eval/`（playbooks/runs 等）。该目录默认定位为临时目录（可不提交），但允许团队按需纳入 git 以共享剧本与基线。
- 依赖
  - 新增 ACP Rust SDK 依赖（`agent-client-protocol`）用于实现 client 侧通信。
  - AI Judge 若启用 OpenAI-compatible 后端，将读取 `OPENAI_*` 环境变量并发起网络请求；该能力必须可选且有清晰的错误提示。
- 安全
  - secrets MUST NOT 出现在 stdout/stderr、playbook 文件、resolved playbook 副本、run artifacts 或日志中；仅允许在内存与 ACP 子进程 env 中存在。
  - terminal 执行与文件访问必须限制在 workspace 内，并提供默认 allowlist/denylist 以降低误用风险。
- 测试
  - 必须提供使用 fake ACP agent 的集成测试，确保 runner/run artifacts 生成可复现，且不依赖真实 claude/codex 安装与登录。

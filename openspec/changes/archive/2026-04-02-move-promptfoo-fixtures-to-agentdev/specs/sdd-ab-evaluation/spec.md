# sdd-ab-evaluation (Delta)

## MODIFIED Requirements

### Requirement: 提供可复现的 SDD prompts Promptfoo 评估套件
SDD workflow MUST 提供一套可复现的 Promptfoo 评估套件，用于对比不同风格/版本的 SDD prompt（baseline vs candidate）。该评估套件（fixtures + 默认模型列表）MUST 存放在仓库顶层 `agentdev/promptfoo/`（而不是 `artifacts/`），并且评估流程 MUST 支持在隔离的临时目录下运行以避免触碰真实用户配置。

#### Scenario: 评估在隔离配置目录下运行
- **WHEN** 维护者运行评测脚本（例如 `bash scripts/sdd-prompts-eval.sh`），并从 `agentdev/promptfoo/` 读取 promptfoo fixtures 与默认模型列表
- **THEN** Promptfoo 产生的数据（例如 `.promptfoo/` 与导出的 `results.*`）均写入该评估流程创建的临时工作目录
- **AND** 不修改用户真实配置目录（仅使用显式指定的 `LLMAN_CONFIG_DIR`）

## ADDED Requirements

### Requirement: Promptfoo fixtures 与 artifacts 目录语义分离
仓库 MUST 使用 `agentdev/` 作为 agent/prompt 相关开发与评测资产的归属目录；`artifacts/` MUST 仅用于测试配置 fixture、schema 产物或其他“可执行/可复用”的非评测资产。Promptfoo fixtures MUST NOT 以长期可执行入口的形式落在 `artifacts/**/promptfoo` 下。

#### Scenario: Promptfoo 评测套件位于 agentdev
- **WHEN** 维护者在仓库中查找 promptfoo 评测套件位置
- **THEN** 评测套件位于 `agentdev/promptfoo/`
- **AND** `artifacts/` 下不再作为 promptfoo fixtures 的稳定入口

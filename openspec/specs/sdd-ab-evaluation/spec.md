# sdd-ab-evaluation Specification

## Purpose
TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive.
## Requirements
### Requirement: Built-In Old-vs-New Evaluation Flow
The SDD workflow MUST provide an evaluation flow that compares legacy and new style outputs on the same scenario set.

#### Scenario: Run evaluation over shared scenarios
- **WHEN** a user executes the evaluation flow for a target scenario set
- **THEN** the system runs both legacy and new style generation/evaluation on equivalent inputs
- **AND** records paired results for comparison

### Requirement: 提供可复现的 SDD prompts Arena 评估套件
SDD workflow MUST 提供一套可复现的 Arena 评估套件，用于对比不同风格/版本的 SDD prompt（baseline vs candidate），并且评估流程 MUST 支持在隔离的 `LLMAN_CONFIG_DIR` 下运行以避免触碰真实用户配置。

#### Scenario: 评估在隔离配置目录下运行
- **WHEN** 维护者设置 `LLMAN_CONFIG_DIR` 为一个仓库内固定测试目录（例如 `./artifacts/testing_config_home`）并运行评估流程
- **THEN** Arena 产生的数据（contest/dataset/run/report）均写入该隔离目录下的 `arena/`
- **AND** 不修改用户真实配置目录

### Requirement: Safety-First Scoring Output
Evaluation outputs MUST prioritize safety and quality signals over cost metrics (for example, token/latency).

#### Scenario: Report includes prioritized metrics
- **WHEN** the evaluation report is generated
- **THEN** it includes quality and safety scores before token/latency metrics
- **AND** it marks pass/fail gates for safety-sensitive checks

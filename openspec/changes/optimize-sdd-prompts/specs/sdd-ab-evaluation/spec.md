## ADDED Requirements

### Requirement: 提供可复现的 SDD prompts Arena 评估套件
SDD workflow MUST 提供一套可复现的 Arena 评估套件，用于对比不同风格/版本的 SDD prompt（baseline vs candidate），并且评估流程 MUST 支持在隔离的 `LLMAN_CONFIG_DIR` 下运行以避免触碰真实用户配置。

#### Scenario: 评估在隔离配置目录下运行
- **WHEN** 维护者设置 `LLMAN_CONFIG_DIR` 为一个仓库内固定测试目录（例如 `./artifacts/testing_config_home`）并运行评估流程
- **THEN** Arena 产生的数据（contest/dataset/run/report）均写入该隔离目录下的 `arena/`
- **AND** 不修改用户真实配置目录

### Requirement: 评估输出优先展示安全与质量信号
SDD prompts 的评估输出 MUST 优先展示与安全/稳定性相关的信号（例如治理字段完整性、STOP/澄清策略可用性）与质量信号，其优先级 MUST 高于 token/延迟等成本类指标。

#### Scenario: 报告按优先级输出指标
- **WHEN** 评估报告生成
- **THEN** 报告在 token/延迟指标之前呈现安全与质量指标

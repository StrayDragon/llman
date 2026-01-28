## MODIFIED Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端中要求明确授权（通过 `--relink-sources`）后才执行来源同步与重链接；未授权时 MUST 退出且不修改来源或托管数据。授权后，完成同步并进入交互式管理器。

#### Scenario: 交互式默认退出
- **WHEN** 用户在交互式终端运行 `llman skills` 且未传入 `--relink-sources`
- **THEN** 命令返回成功并且不执行同步、复制、或重链接

#### Scenario: 交互式授权后继续
- **WHEN** 用户在交互式终端运行 `llman skills --relink-sources` 且确认
- **THEN** 命令执行同步流程并进入交互式管理器

## ADDED Requirements
### Requirement: 交互式重链接确认与跳过
交互式终端下，`llman skills --relink-sources` MUST 提示用户确认来源重链接，默认答案为否；若传入 `--yes` 则跳过确认并直接执行同步。

#### Scenario: 默认拒绝
- **WHEN** 交互式确认提示显示且用户选择否或直接取消
- **THEN** 命令退出且不修改来源或托管数据

#### Scenario: --yes 跳过确认
- **WHEN** 用户传入 `--yes`
- **THEN** 命令不显示确认提示且执行同步流程

### Requirement: 非交互模式需显式授权
非交互终端下，`llman skills` MUST 要求传入 `--relink-sources` 才能执行来源同步与重链接；未传入时 MUST 返回错误且不修改来源或托管数据。

#### Scenario: 非交互缺少授权
- **WHEN** 用户在非交互终端运行 `llman skills` 且未传入 `--relink-sources`
- **THEN** 命令返回错误且不执行同步、复制、或重链接

#### Scenario: 非交互授权后继续
- **WHEN** 用户在非交互终端运行 `llman skills --relink-sources`
- **THEN** 命令执行同步流程并更新目标链接

## MODIFIED Requirements
### Requirement: Explicit confirmation for rc writes
completion install MUST 在修改 shell rc/profile 文件前提示确认，且当用户拒绝时 MUST 不写入。若处于非交互环境，命令 MUST 拒绝写入并返回错误，除非显式提供 `--yes`；当提供 `--yes` 时 MUST 直接执行写入且不出现交互提示。

#### Scenario: 拒绝确认不会产生副作用
- **WHEN** 用户拒绝确认提示
- **THEN** 不修改任何 rc/profile 文件

#### Scenario: 非交互 install 未提供 --yes
- **WHEN** 命令在非交互环境运行，且包含 `--install` 但未提供 `--yes`
- **THEN** 命令以非零退出，且不修改任何 rc/profile 文件

#### Scenario: 非交互 install 提供 --yes
- **WHEN** 命令在非交互环境运行，且包含 `--install --yes`
- **THEN** completion block 被安装/更新，且命令成功退出

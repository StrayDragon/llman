## Why

`llman sdd validate --specs` 对所有 spec 报「passed」,但只做了 Gherkin 结构校验,从不跑 BDD runner。这给维护者虚假的安全感——feature 文件即使所有 step 都无框架绑定,fast mode 照样「valid」。

当前 `--check` 是 opt-in 且仅单 spec 路径可用。Bulk 路径 (`--specs`) 硬编码 `check_mode=false`,永远不触发 BDD 执行。

当 `bdd.run_command` 已配置时,BDD 框架自身就是最好的 step 覆盖率检查器——未绑定的 step 文本框架必报错。我们不需要在 llman 里做跨框架 pattern 扫描。

## What Changes

`validate` 行为变更: **当 `bdd.run_command` 已配置时,自动运行 BDD check**。

- `validate --specs` 和 `validate <spec-id>` 在 BDD-on 时默认跑 `bdd.run_command`,框架自行报告结果
- 新增 `--no-check` 跳过 BDD runner(恢复旧行为)
- `validate --changes` / `validate <change-id>` 不受影响(changes 无 BDD)
- BDD-off(未配 `bdd:` 段)行为不变

## Capabilities

- 移除 `--check` 在无 bdd 配置时的允许(当前允许但无效果,产生误导)
- 新增 `--no-check` flag

## Impact

- **破坏性变更**: 有 `bdd:` 配置的项目,`validate` 耗时增加(需要跑 BDD runner)
- 用户可通过 `--no-check` 快速跳过
- BDD-off 项目完全不受影响

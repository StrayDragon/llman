## Why
当前 `llman skills` 的交互流程是先选技能，再逐个进入目标切换，使用中会重复进入目标菜单且不便按 agent 视角操作。用户希望按 agent 目标为中心操作：先选择目标，再基于该目标的现有链接状态进行技能勾选与差异同步。同时要求 registry 仅记录用户确认后的状态，不自动吸收外部变动。

## What Changes
- **BREAKING** 交互流程调整为“先选单个 target → 再选技能 → 确认后按 diff 应用”。
- 交互默认勾选来源改为目标目录的实际软链接状态（filesystem）。
- registry 仅在用户确认应用后写入，不作为交互默认来源。
- 更新文案与测试以覆盖新流程。

## Impact
- 受影响规范：`skills-management`
- 受影响代码：`src/skills/command.rs`、`src/skills/sync.rs`、`src/skills/registry.rs`、`locales/app.yml`、`tests/skills_integration_tests.rs`

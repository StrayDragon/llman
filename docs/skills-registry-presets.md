# Skills 分组推断与实时状态说明

本文档说明 `llman skills` 在移除 `registry.json` 依赖后的行为。

## 关键变化

- `llman skills` 不再读取或写入 `<skills_root>/registry.json`。
- 技能启用状态基于目标目录真实链接状态实时计算。
- 分组（preset）仅按技能目录名推断，不再支持配置化 `presets`（如 `extends`、`description`、`skill_dirs`）。

## 状态计算规则

- 交互模式：默认勾选来自目标目录真实链接状态。
- 非交互模式：
  - 已链接技能优先保持启用；
  - 未链接技能按 `config.toml` 中对应 target 的 `enabled` 默认值决定是否启用。

## 分组推断规则

运行 `llman skills` 时，分组仅由目录名推断：

- `<group>.<name>` 会归入 `<group>`
- 不包含 `.` 的目录会归入 `ungrouped`

示例目录：

- `superpowers.brainstorming`
- `astral-sh.uv`
- `mermaid-expert`

对应分组：

- `superpowers`
- `astral-sh`
- `ungrouped`

## 迁移说明（从旧 registry 配置）

若你之前在 `registry.json` 维护了 `presets`：

- 该文件会被忽略，不再生效；
- 请将分组语义迁移到目录命名（`<group>.<skill>`）上。

例如旧配置中：

- `daily` 包含 `superpowers.brainstorming`
- `daily` 包含 `mermaid-expert`

可以改为目录命名：

- `daily.brainstorming`
- `daily.mermaid-expert`

## 建议

- 优先通过目录结构管理分组。
- 修改后执行一次 `llman skills`，确认交互树分组是否符合预期。

## Why

当前 `llman skills` 同时依赖文件系统链接状态和 `<skills_root>/registry.json`，造成状态来源分裂、行为理解成本高、维护面大。我们希望将状态收敛为运行时实时计算，以减少持久化状态漂移与兼容负担。

## What Changes

- 移除 `llman skills` 对 `<skills_root>/registry.json` 的读写依赖，不再把技能启用状态持久化到 registry。
- 交互与非交互流程统一改为基于目标目录真实链接状态与 `config.toml` 默认值进行实时计算。
- 预设来源调整为运行时目录分组推断（`<group>.<name>`），不再支持 `registry.presets` 的配置化预设（`description`/`extends`/`skill_dirs`）。
- **BREAKING**：已有 `registry.json` 将不再被读取或更新；依赖 `registry.presets` 的分组配置将失效，需要迁移到目录命名约定。

## Capabilities

### New Capabilities

- （无）

### Modified Capabilities

- `skills-management`: 将技能状态与预设来源从 registry 持久化机制改为实时计算机制，并更新非交互同步语义与相关错误/兼容要求。

## Impact

- 受影响规范：`openspec/specs/skills-management/spec.md`
- 受影响代码（预期）：
  - `src/skills/config/mod.rs`
  - `src/skills/catalog/registry.rs`
  - `src/skills/catalog/types.rs`
  - `src/skills/cli/command.rs`
  - `src/skills/targets/sync.rs`
  - `tests/skills_integration_tests.rs`
  - `docs/skills-registry-presets.md`
  - `README.md`
- 用户影响：`skills` 行为对文件系统状态更一致，但需要接受 `registry.presets` 下线与旧 registry 文件失效。

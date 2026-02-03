# Design: SDD/Skills 模块结构重组（方案 C）

## Context
- `src/sdd/` 与 `src/skills/` 当前是扁平模块集合，职责边界不清晰，影响维护与扩展。
- 本次重构只改变模块布局与内部依赖路径，不改变 CLI 行为、配置格式、输出 JSON 结构。
- 已补充并运行行为合同测试作为重构“保护网”。

## Goals
- 按领域切片组织 `sdd` 与 `skills` 模块，提升导航与边界清晰度。
- 仅保留有意义的对外接口，避免为兼容性做过量 re-export；必要的调用路径直接适配。
- 使共享工具与核心逻辑的依赖方向清晰、避免循环。

## Non-goals
- 不新增或删除 CLI 子命令/参数。
- 不修改模板/配置文件内容与 JSON 输出格式。
- 不改变目录输出、链接策略或 registry 格式。

## Proposed Module Layout

### SDD
目标：以 change/spec/project/shared 四个领域切片，保留 `command.rs` 作为入口。

Proposed tree:
```
src/sdd/
  command.rs
  mod.rs
  change/
    archive.rs
    delta.rs
    list.rs
    show.rs
    validate.rs
  spec/
    list.rs
    show.rs
    validate.rs
    parser.rs
    validation.rs
    staleness.rs
  project/
    init.rs
    update.rs
    update_skills.rs
    config.rs
    templates.rs
    regions.rs
    fs_utils.rs
  shared/
    constants.rs
    discovery.rs
    interactive.rs
    match_utils.rs
    item.rs
```

Notes:
- `command.rs` 仍保留在 `src/sdd/` 根部，保持 CLI 入口清晰。
- `list/show/validate` 作为跨 change/spec 的命令实现归入 `shared`，避免拆分带来的重复。
- `templates.rs` 迁移后更新 `include_str!` 路径（改用 `CARGO_MANIFEST_DIR` 拼接路径）。

### Skills
目标：以 CLI / catalog / targets / config / shared 切分。

Proposed tree:
```
src/skills/
  mod.rs
  cli/
    command.rs
    interactive.rs
  catalog/
    scan.rs
    registry.rs
    types.rs
  targets/
    sync.rs
  config/
    config.rs
  shared/
    git.rs
```

Notes:
- 不保留兼容性 re-export；调用方直接改用新的模块路径。
- `find_git_root` 作为共享工具保留在 `skills::shared`，调用方改用新路径引用。

## Compatibility Strategy
- **Public API**：避免为兼容性做 re-export；更新调用方代码以适配新路径。
- **CLI 行为**：所有 CLI 行为由合同测试锁定；重构时以测试为回归基准。
- **Template include paths**：在移动 `templates.rs` 后统一更新 `include_str!` 路径，避免编译期失败。

## Risks & Mitigations
- **Risk**: `include_str!` 相对路径失效
  - **Mitigation**: 改为基于 `CARGO_MANIFEST_DIR` 的绝对路径拼接或逐一修正路径。
- **Risk**: re-export 不完整导致外部调用失败
  - **Mitigation**: 在 mod.rs 中显式列出对外接口；编译与 tests 验证。
- **Risk**: use 路径遗漏导致编译失败
  - **Mitigation**: 小步移动文件并持续 `cargo check`/`just test`。

## Decision Records
- 使用领域切片而非分层目录：更贴近业务概念（change/spec/project/skills），降低认知成本。
- 保持入口命令路径不变：减少 CLI 入口与外部依赖的改动面。

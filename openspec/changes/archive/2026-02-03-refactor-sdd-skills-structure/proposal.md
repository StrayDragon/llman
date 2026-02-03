## Why
- `src/sdd/` 与 `src/skills/` 当前为扁平模块，跨文件跳转频繁、边界感弱，影响维护与新功能演进。
- 本次是跨多个子系统的大型重构，先通过行为合同测试锁定 CLI 输出与交互，再做结构调整更安全。
- 采用按领域切片的结构（方案 C）能降低认知成本，并为后续功能扩展保留清晰边界。

## What Changes
- 将 `src/sdd/` 重组为按领域划分的子模块（变更/change、规范/spec、项目/project、共享/shared），并直接适配调用路径，避免为兼容性做过量 re-export。
- 将 `src/skills/` 重组为按领域划分的子模块（CLI、发现/registry、targets/link、配置、共享），并直接适配调用路径。
- 调整内部 `use` 路径与 `include_str!` 相对路径，使模板加载与 CLI 行为保持不变。
- 不改变 CLI 参数、输出格式、行为、配置文件格式；仅做代码结构与模块边界整理。

## Impact
- 受影响规范：`sdd-workflow`、`skills-management`（行为保持一致）。
- 受影响代码：`src/sdd/**`、`src/skills/**`、`src/prompt.rs`（若 shared helper 位置调整）。
- 主要风险：`include_str!` 相对路径遗漏、re-export 不完整导致编译失败、内部 `use` 路径遗漏。
- 缓解手段：已补充并运行 SDD/Skills 行为合同测试 + 全量 `just test`。

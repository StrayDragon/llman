## Why
`llman tool clean-useless-comments` 当前存在可用性与规则语义一致性问题：
- 当用户通过 `files...` 显式传入路径时，目录会被当作“存在的输入”加入处理队列，后续 `read_to_string` 失败会表现为噪音错误，用户难以理解。
- doc comment（docstrings/jsdoc/doc-comments/godoc）开关语义与 line/block comment 的开关语义不一致，配置 `true` 时反而可能“不移除”，存在理解偏差风险。
- 大规模扫描时 include/exclude glob 每次匹配都重新编译，带来不必要开销。

### Current Behavior（基于现有代码）
- 显式 files：仅检查 `exists()`（`src/tool/processor.rs`），不区分文件/目录。
- doc comment：`Some(true)` 分支当前返回“不移除”（`src/tool/tree_sitter_processor.rs`），语义疑似反向。
- glob：每次匹配时 `glob::Pattern::new(...)`（`src/tool/processor.rs`），重复编译。

## What Changes
- 对显式传入的 `files...`：遇到目录/非普通文件时明确跳过并输出清晰提示，不将其计为文件读取失败。
- 统一 doc comment 开关语义：与其它注释开关一致（`true`=允许移除，`false`=禁止移除，`None`=默认保留）。
- 性能：预编译 include/exclude glob，一次运行复用，减少重复编译成本。

### Non-Goals（边界）
- 不改变 tree-sitter 不可用时的“安全失败”策略（仍按现有 spec：跳过修改并记录错误）。
- 不启用 regex fallback（仍保持默认禁用）。

## Impact
- Affected specs: `specs/tool-clean-comments/spec.md`
- Affected code: `src/tool/processor.rs`, `src/tool/tree_sitter_processor.rs`, `src/tool/config.rs`

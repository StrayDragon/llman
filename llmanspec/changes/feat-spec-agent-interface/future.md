# Future Items

## Deferred: Implementation

所有 tasks.md 中的实现任务均被延期，等待设计确认后按 Phase 顺序实施。

触发条件：
- design.md 中关于 embedding API 调用方式（Python helper vs Rust HTTP）的设计决策完成
- 确认 coral API 的长期可用性或切换到 fastembed

## Later: Health Detection

- 本 change 的 `list --json --meta` 预留了 health 字段，但具体检测逻辑在 `feat-spec-quality-triage` 中实现

## 1. Resolver 收敛

- [x] 1.1 删除 `src/config.rs` 中的 macOS legacy 探测、回退与 warning 分支，保持 CLI/env override 优先级与 `~/.config/llman` 默认路径不变。
- [x] 1.2 清理仅服务于 legacy fallback 的 helper、枚举与 locale 文案，确保解析阶段仍不创建目录。

## 2. 验证与发布说明

- [x] 2.1 调整 `src/config.rs` 单元测试与相关 CLI / 集成测试，覆盖“旧 macOS 目录存在时默认仍使用 `~/.config/llman`”的行为。
- [x] 2.2 运行 `openspec validate --strict` 与相关 Rust 检查，并在变更说明中明确这是一次 breaking 的默认路径收敛。

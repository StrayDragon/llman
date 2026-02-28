## 1. 规范对齐与告警文案

- [x] 1.1 为 “macOS legacy 配置目录被使用” 增加一个稳定的 stderr 警告文案（建议走 i18n key），包含 legacy 路径与建议迁移目标 `~/.config/llman`。
- [x] 1.2 在代码中明确 “recognizable config root” 的判定标准（例如：存在 `config.yaml` 或存在 `prompt/` 目录），并与 `specs/config-paths/spec.md` 保持一致。

## 2. 手写 resolver：默认统一 `~/.config/llman`

- [x] 2.1 实现内部 `home_dir()`（不依赖 `directories/dirs`），并在缺失时返回清晰错误（复用现有 `errors.home_dir_missing` 语义）。
- [x] 2.2 将 `resolve_config_dir_with` 的默认分支从 `ProjectDirs` 替换为 `<home>/.config/llman`（保持 precedence：CLI > ENV > default；保持“解析阶段不创建目录”）。
- [x] 2.3 调整 `~` 展开逻辑，改用新的 `home_dir()`，并保持现有单测覆盖（CLI/env 的 tilde expansion）。

## 3. macOS legacy 兼容：检测 + 选择 + 警告

- [x] 3.1 在无 CLI/env override 时，实现 macOS legacy 目录候选检测：
  - `<home>/Library/Application Support/llman`
  - `<home>/Library/Application Support/com.StrayDragon.llman`
- [x] 3.2 实现选择规则：优先 `~/.config/llman`（若包含 recognizable config root），否则在 legacy 包含 recognizable config root 时回退到 legacy，并输出迁移警告（stderr）。
- [x] 3.3 确保 legacy 回退发生在 `ensure_global_sample_config` 之前，避免在 legacy 存在时提前创建新目录/写入 sample config。

## 4. 移除依赖 crates 并统一子模块路径

- [x] 4.1 从 `Cargo.toml` 移除 `directories` 与 `dirs`（以及对应的使用点），并以内部 helper 替换所有调用（至少覆盖当前仓库所有引用点）。
- [x] 4.2 让 `x/codex`、`x/claude-code` 等子模块的 config file path 统一基于同一 resolver（避免各自 fallback 到不同默认目录）。

## 5. 回归测试与文档

- [x] 5.1 更新现有 `src/config.rs` 单测：默认路径不再断言 `ProjectDirs`，改为断言 `<home>/.config/llman`（使用 `TestProcess`/`TempDir` 控制 HOME）。
- [x] 5.2 为 legacy 选择逻辑补充单测：在不依赖真实 macOS 的情况下可验证决策（建议把“选择逻辑”抽成纯函数并对其测试）。
- [x] 5.3 更新用户可见文档/帮助：明确默认目录为 `~/.config/llman`，并记录 macOS legacy 兼容与迁移建议。

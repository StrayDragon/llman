## Context

当前配置目录解析逻辑已经完成“默认路径统一到 `~/.config/llman`”的长期目标，但仍保留一层仅针对 macOS 的 legacy fallback：
- `src/config.rs` 里通过 `is_recognizable_config_root`、`MacosConfigDirChoice` 和 `select_macos_config_dir_choice` 检测两个历史目录；
- 当默认目录没有可识别配置、而旧目录存在配置时，resolver 会改为返回旧目录；
- 同时输出 `messages.macos_legacy_config_dir_warning` 提示迁移。

这层逻辑现在主要带来维护成本：默认行为不再是单纯的“解析为固定路径”，而是依赖文件系统历史状态；测试需要覆盖多目录组合；用户也更难理解为什么不同机器会解析到不同默认路径。

## Goals / Non-Goals

**Goals:**
- 将 Linux/macOS 无 override 时的默认配置目录行为固定为 `~/.config/llman`。
- 删除 macOS legacy 自动探测、自动回退和迁移 warning 的实现与测试负担。
- 保持现有优先级不变：CLI `--config-dir` > `LLMAN_CONFIG_DIR` > 默认路径。
- 保持显式 override 能力，使仍依赖旧路径的用户可以自行指定目录。

**Non-Goals:**
- 不新增 `$XDG_CONFIG_HOME` 解析或新的路径规则。
- 不自动迁移旧目录内容到 `~/.config/llman`。
- 不改变 Windows 的 home 目录探测逻辑。
- 不重构 `Config` / prompt 存储结构。

## Decisions

### 1) 删除 macOS 特化选择器，默认路径直接返回 `~/.config/llman`
- 在 `resolve_config_dir_with()` 中保留 CLI/env override 与 `~` 展开逻辑不变。
- 删除“默认目录是否可识别”“旧目录是否可识别”的三路选择逻辑。
- 无 override 时直接返回 `<home>/.config/llman`，并继续保持“解析阶段不创建目录”的约束。

之所以直接删除而不是保留 dormant 分支，是因为 legacy fallback 已不再是产品行为的一部分；继续保留只会让 resolver 继续承担状态机复杂度。

### 2) 旧目录仅通过显式 override 访问
- 历史目录不再参与默认解析。
- 若用户仍需读取旧目录，必须显式传入 `--config-dir` 或设置 `LLMAN_CONFIG_DIR`。

这能保留手动逃生口，同时避免默认行为继续“猜测”用户想要哪套目录。

### 3) 清理配套 helper、warning locale 与测试
- 删除 `is_recognizable_config_root`、`MacosConfigDirChoice`、`select_macos_config_dir_choice` 及相关单测。
- 删除 `messages.macos_legacy_config_dir_warning` locale。
- 将测试重点改为：
  - override 优先级保持不变；
  - 默认路径恒定为 `~/.config/llman`；
  - 即使旧 macOS 目录存在，默认解析也不再切换。

## Risks / Trade-offs

- [仍有少量旧用户依赖自动 fallback] → 这些用户升级后会看到默认配置变为 `~/.config/llman`
  → 缓解：在 proposal / 发布说明里明确 BREAKING，并说明可以使用 `--config-dir` 或 `LLMAN_CONFIG_DIR` 指向旧目录。

- [不再提供运行时迁移提醒] → 用户可能不会被动收到迁移提示
  → 缓解：用 release notes / changelog 承担一次性告知，而不是持续在运行时保留复杂逻辑。

- [删除 helper 后，测试需要重写] → 现有 macOS choice 单测会失效
  → 缓解：改写为对公共 resolver 行为的断言，减少实现细节测试。

## Migration Plan

1. 更新 `config-paths` spec，明确移除 legacy fallback/warning。
2. 简化 `src/config.rs` resolver，并清理不再使用的 helper、warning locale 和相关测试。
3. 运行 OpenSpec 校验与配置路径相关 Rust 测试。
4. 在发布说明中标注 BREAKING：默认解析不再自动读取 macOS 旧目录；如仍需旧目录，用户需显式 override。

回滚策略：若发布后需要恢复兼容，可回退到本变更前的 resolver 与 locale，同时恢复 legacy 测试覆盖。

## Open Questions

- None.

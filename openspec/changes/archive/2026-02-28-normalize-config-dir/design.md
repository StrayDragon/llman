## Context

当前配置目录解析集中在 `src/config.rs`：

- 解析优先级：CLI `--config-dir` > `LLMAN_CONFIG_DIR` > `ProjectDirs::config_dir()`（平台相关）
- `~` 展开依赖 `dirs::home_dir()`
- CLI 启动时会将解析结果写回 `LLMAN_CONFIG_DIR`，并调用 `ensure_global_sample_config` 在该目录下生成 `config.yaml`（若不存在）

这导致 macOS 的默认目录落到 `~/Library/Application Support/...`，与当前 CLI help/团队预期的 `~/.config/llman` 不一致；且历史版本已在旧目录产生真实用户配置，直接切换默认路径会破坏旧用户。

此外，部分子模块（例如 `x/codex`、`x/claude-code`）在未设置 `LLMAN_CONFIG_DIR` 时也使用 `ProjectDirs` 作为 fallback，进一步放大了平台差异与路径分叉。

## Goals / Non-Goals

**Goals:**
- macOS/Linux 默认统一使用 `~/.config/llman`（保持可通过 CLI/env 覆盖）
- 保持现有解析优先级与“解析阶段不创建目录”的不变式
- macOS 对 legacy 目录提供兼容解析，并对用户给出迁移警告
- 移除 `directories/dirs` 依赖，改为手写 resolver，减少平台差异与不可控行为
- 让所有子模块使用同一套 resolver，避免多处各自解析导致的再次分叉

**Non-Goals:**
- 本变更不自动迁移/移动用户文件（只警告 + 兼容解析）；后续可单独引入迁移命令
- 不扩展 Windows 支持（仍保持“部分支持”的现状，且主要覆盖 macOS/Linux）

## Decisions

### 1) 默认配置目录改为固定路径（macOS/Linux）

将“无 CLI/env override 时的默认目录”从 `ProjectDirs` 改为固定的：

- `<home>/.config/llman`

其中 `<home>` 由手写的 `home_dir()` 解析得到（优先使用 `$HOME`，为空或缺失则报错并沿用现有错误语义）。

理由：
- 与现有 CLI help 文案一致
- 对用户与排障更可预测
- 不再依赖 `ProjectDirs` 的平台差异行为

### 2) macOS legacy 目录兼容：检测 + 警告 + 自动解析

仅在“未提供 CLI/env override（即使用默认路径）”的情况下启用 legacy 兼容逻辑，以避免覆盖用户显式意图。

legacy 目录候选：
- `~/Library/Application Support/llman`（历史 `ProjectDirs::from(\"\", \"\", \"llman\")`）
- `~/Library/Application Support/com.StrayDragon.llman`（历史 `ProjectDirs::from(\"com\", \"StrayDragon\", \"llman\")`，用于部分子模块）

选择规则（确定性优先）：
- 若 `~/.config/llman` 已存在且包含可识别的配置（例如 `config.yaml` 或 `prompt/`），优先使用它。
- 否则，若存在 legacy 目录且包含可识别的配置，则解析结果使用 legacy 目录，并在启动时输出迁移警告（stderr）。
- 若两者都不存在，则使用 `~/.config/llman` 作为解析结果（后续由现有逻辑在需要时创建目录/生成 sample config）。

理由：
- 既能让新用户走统一路径，也能避免旧用户配置“突然失效”
- 避免在 legacy 存在时提前创建新目录/写入 sample config，造成未来选择反转或混淆

### 3) 全仓库统一复用 resolver

移除各处 `ProjectDirs` fallback，统一通过 `crate::config::resolve_config_dir(..)`（或其新的内部实现）来得到配置根目录，并在其下派生文件路径（如 `codex.toml`、`claude-code.toml`）。

理由：
- 单一真源（single source of truth），减少路径分叉
- macOS legacy 兼容规则集中实现，避免遗漏

### 4) 移除 `directories/dirs` crates

实现一个内部的“路径解析层”，提供至少：
- `home_dir()`：解析 home
- `default_config_dir()`：生成 `<home>/.config/llman`
- `legacy_macos_config_dir_candidates(home)`：生成 legacy 候选列表（仅 macOS 编译）

并逐步替换所有对 `directories/dirs` 的调用点（包括但不限于：`src/config.rs`、`src/self_command.rs`、`src/prompt.rs`、`src/skills/config/*`、`src/x/*` 等）。

## Risks / Trade-offs

- [用户已有两套目录] → 规则必须确定且可解释；在警告中明确当前使用的路径，并给出迁移建议（优先迁到 `~/.config/llman`）。
- [默认路径变化带来隐式“破坏性”] → 仅对 macOS 做 legacy fallback，且仅在无显式 override 时触发；保持旧用户可用。
- [移除依赖导致边界行为变化] → 用单测锁定 precedence / tilde expansion / legacy fallback；HOME 缺失时返回清晰错误。
- [测试与本地开发误触真实配置] → 保持现有 dev project guard 与 `LLMAN_CONFIG_DIR` 约束不变。

## Migration Plan

- 第 1 阶段（本变更）：仅警告 + 自动解析 legacy，不自动搬迁文件。
- 用户迁移建议（文档/警告提示中给出）：将 legacy 目录中的 `config.yaml`、`prompt/`、以及相关 `.toml` 文件复制到 `~/.config/llman`，完成后可删除 legacy。
- 回滚策略：若出现严重兼容问题，用户可通过 `--config-dir` 或 `LLMAN_CONFIG_DIR` 显式指定 legacy 目录。

## Open Questions

- “可识别的配置”检测条件是否应覆盖更多文件（例如仅有 `prompt/` 没有 `config.yaml` 的情况）。
- 当 `~/Library/Application Support/llman` 与 `~/Library/Application Support/com.StrayDragon.llman` 同时存在且内容不一致时，是否需要更细粒度的合并策略（本轮先不做合并，只做目录级选择与警告）。

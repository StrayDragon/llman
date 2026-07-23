---
depends_on: []
branch: feat/add-sdd-config-skills-command
base_sha: 87de7e7e24e4f26e18d36f691103faae0472a8d6
checkpointed: true
checkpoint_sha: 87de7e7e24e4f26e18d36f691103faae0472a8d6
---

# Proposal: Add `llman sdd config` interactive command

## Why

项目现在有 8 个 optional SDD skill（5 个官方 + 3 个新加的 arch-review/wayfinder/research），但管理 `extra_skills` 只能手工编辑 `config.yaml` + 记住合法值。且 3 个新 skill 尚未被 `init --update` 托管（不在 `OPTIONAL_SKILL_NAMES` 白名单，写入 `extra_skills` 会报错）。

需要一个交互式命令让用户方便地查看与勾选可选 skill，同时把 3 个新 skill 纳入官方可选集（常量 + 模板 + schema）使其能被托管。

## What Changes

### 1. 纳入 3 个新 skill 到官方可选集
- `OPTIONAL_SKILL_NAMES`（config.rs）+ `OPTIONAL_SKILL_FILES`（templates.rs）各加 3 项。
- 新增 6 个模板（en + zh-Hans）。
- 重生成 schema（`extra_skills` 合法值含 8 项）。
- 同步 `DEFAULT_CONFIG_*` 注释示例。

### 2. 新增 `llman sdd config`（总览，r109）
- 无子命令时打印当前 config 摘要（schema/locale/extra_skills 启用数/bdd on|off/archive）。

### 3. 新增 `llman sdd config skills`（交互式管理，r110）
- `inquire::MultiSelect` 展示全部 8 个候选（每项附一行描述），当前 extra_skills 为默认勾选。
- 确认后原子写回（接受丢注释，复用 `write_config`）。
- `--no-interactive` 降级为打印当前启用 + 可选未启用列表。
- `--json` 输出 `{enabled, available}`。
- ESC/取消 = 安全 no-op。

## Capabilities

| Capability | Change | Type |
|------------|--------|------|
| `sdd-workflow` | config 命令 + OPTIONAL_SKILL_NAMES 扩展 | add + modify |

## Impact

- **CLI**: 新增 `config` 子命令组（含 `config skills`）。不改变现有命令行为。
- **config**: `extra_skills` 合法值从 5 扩到 8。
- **templates**: 新增 6 个 skill 模板文件。
- **schema**: 重生成（enum 扩展）。
- **Breakage**: 无。现有 5 个官方 optional skill 行为不变；3 个新 skill 从「手动放置易被 init --update 清理」变为「可托管」。

## Design Decisions

见 `design.md`。核心：MultiSelect（非 ratatui tui_picker）、接受丢注释、纳入新 skill 需同步常量+模板+schema。

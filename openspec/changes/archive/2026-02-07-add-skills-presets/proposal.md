# Proposal: add-skills-presets

## Why

当前 `llman skills` 在技能较多时需要反复手动勾选，操作成本高、容易漏选。与此同时，用户已经通过技能目录命名（如 `<preset>.<skill>`）形成了默认分组习惯，但工具尚未把这种命名约定转化为可直接应用的预设体验。

本提案目标是在**不引入新命令参数**的前提下，把预设能力收敛为交互式流程：
- 允许从 `registry.json` 读取用户手工维护的预设
- 当用户未自定义预设时，自动从技能目录命名推断默认预设（仅运行时，不写回 registry）
- 在 `llman skills` 启动时完成预设合法性检查，发现问题立即报错

## What Changes

- 在 `registry.json` 中新增可选 `presets` 字段（只读消费，用户自行维护）
- 新增默认预设推断：当 `presets` 为空时，扫描 `<skills_root>` 下目录名并按 `<preset>.<skill>` 规则自动生成运行时预设
- 新增预设解析与继承（`extends`）能力，支持去重与循环检测
- 将预设能力限定为交互模式，不新增 `--preset`、`--save-preset`、`--list-presets` 等 CLI 参数
- 交互流程新增模式选择：`Apply preset` / `Select individually`
- `llman skills` 进入交互前先执行预设校验（父预设存在、无循环、引用技能存在），失败即中止
- 技能列表继续支持按目录名前缀分组展示（`group.name` → `group`）

## Capabilities

### New Capabilities
- `skills-presets-interactive`: 交互式预设应用与默认预设推断

### Modified Capabilities
- `skills-management`: 交互入口从固定三段式扩展为“模式选择 + 后续流程”

## Impact

- 受影响 specs: `skills-management`
- 受影响文件（预期）:
  - `src/skills/catalog/registry.rs`
  - `src/skills/cli/command.rs`
  - `src/skills/catalog/types.rs`
  - `locales/*.yml`（新增交互与报错文案）
  - `tests/*skills*_tests.rs`

## Non-Goals

- 不提供预设 CRUD 子命令或参数
- 不新增 JSON 输出协议
- 不自动修改或回写用户 `registry.json` 中的 `presets`

## Risks

- R1: 自动推断默认预设可能暴露命名不规范问题
  - Mitigation: 启动时做严格校验并给出可定位错误
- R2: 继承链配置错误导致交互不可用
  - Mitigation: 在进入任何交互前 fail-fast

## Success Criteria

1. 用户执行 `llman skills` 时，可在交互模式下选择并应用预设
2. 未配置 `registry.presets` 时，工具可从 `<preset>.<skill>` 命名自动推断默认预设
3. 发现无效预设配置时，命令在交互前即报错退出
4. 本变更不引入新的 presets CLI 参数

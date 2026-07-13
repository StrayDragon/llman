# Proposal: SDD Status Compact & Apply-Cycle Skill

## Why

当前 `llman sdd status` 输出过于简单（只有聚合计数），agent 拿到后仍需手动 `read tasks.md`、`read spec.toon`、`read proposal.md` 才能了解上下文。每次 skill 调用和文件读取造成大量 token 浪费。

openspec 时期的经验表明：**一个紧凑的 command 输出 + 一个不可中断的单闭环 skill** 能大幅减少 token 消耗并提高执行确定性。

## What Changes

### 1. 重写 `llman sdd status` 为"握把 + 指导"

- 接受可选 `[TARGET]` 参数（change 名称、归档日期前缀、部分名称模糊匹配）
- 默认输出纯 TOON 格式（agent 可直接解析，无需混搭 markdown/mermaid）
- 移除旧的纯文本 human-readable 输出
- 极度紧凑：只显示未完成任务/pending ops，不显示已完成项
- TOON 输出尾部包含 `next` 字段作为明确的行动指导
- `--json` 保持 JSON 格式（向后兼容）
- 新增 `--format toon|json` 参数，`toon` 为默认值, `--json` 等价于 `--format json`

### 2. 新增 `llman-sdd-apply-cycle` 手动触发 skill

- 吸收 openspec 的"单闭环"模式
- `disable-model-invocation: true` → 不出现在 `available_skills` 中，agent 不能自动启用
- 用户通过 `/skill:llman-sdd-apply-cycle` 手动触发
- 合并 apply → verify → archive → commit 为一个不可分割的闭环
- 不修改/移除现有 `llman-sdd-apply/verify/archive` skills

## Capabilities

| Capability | Change | Type |
|------------|--------|------|
| `cli` | Status command: new args, compact markdown output | modify + add |
| `sdd-workflow` | Apply-cycle workflow, `disable-model-invocation` skill | add |

## Impact

- **Token**: 每次 SDD 开发循环预计节省 5K-8K tokens（~50%）
- **CLI**: 向后兼容，新增参数和输出格式，不改变现有行为
- **Skills**: 新增 1 个手动 skill，不修改现有 8 个 skill
- **Breakage**: 无。`--json` 输出格式不变，`llman sdd list`/`show` 不变

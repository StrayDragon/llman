# Design: Spec Quality & Agent Triage

> 本 change 基于 `feat-spec-agent-interface` 完成（context 命令 + index 机制已可用）。
> 所有 prompt/skill 模板调整依赖 context 命令的可用性。

## 1. Skill 模板调整策略

### 1.1 调整范围

| 文件 | 改动类型 | 影响 |
|------|---------|------|
| `onboard.md` | 插入步骤 2-4 | 新用户 onboarding 流程 |
| `explore.md` | 修改建议动作、退出选项 | 探索模式体验 |
| `propose.md` | 首步插入 triage | 提案流程 |
| `apply.md` | 首步插入 context | 实施流程 |
| **(新)** `quick.md` | 新增文件 | 快速路径 |
| `sdd-commands.md` | 追加 2 行 | 命令参考 |
| `structured-protocol.md` | Constraints + Workflow 各加 1 条 | 全局约束 |

### 1.2 注入方式

所有调整通过模板单元注入（`{{ unit(...) }}`）实现，不修改 Jinja 渲染引擎。

- `onboard.md` / `explore.md` / `propose.md` / `apply.md`：修改步骤顺序，新增步骤
- `quick.md`：全新文件，注册到 skills 模板目录
- `sdd-commands.md`：追加命令参考条目
- `structured-protocol.md`：在现有 Constraints 和 Workflow 列表中追加条目

这些改动在 `llman sdd update-skills` 时生效。

### 1.3 与 `feat-spec-agent-interface` 的依赖关系

```
feat-spec-agent-interface          feat-spec-quality-triage
        │                                   │
        ├─ context 命令                    ├─ validate --health (future)
        ├─ index rebuild 命令               ├─ skill 模板调整 ← 依赖上下文就绪
        ├─ list --json --meta               │   └─ 模板引用 context 命令
        └─ 异步重建机制                      ├─ triage 规则
                                            ├─ quick path skill
                                            └─ 结构化协议约束
```

skill 模板中引用的 `llman sdd context` / `llman sdd index rebuild` 命令在 `feat-spec-agent-interface` 实现后才可用。但模板本身可以独立部署——agent 看到 context 命令不可用时走 error handling 路径即可。

## 2. Validate --Health（延期实现）

`validate --health` 的坏口味检测（僵尸 req、迷雾 spec、范围膨胀、重复规范）将在独立 change 中实现。本 change 只做：

- `list --json --meta` 中预留 `health` 字段（由 `feat-spec-agent-interface` 实现）
- skill 模板中不引用 `validate --health`，避免对未实现功能的引用

## 3. Skill 模板语言注意事项

### 3.1 context 命令的引用方式

所有模板中引用 context 命令时，使用**建议性语言**而非强制要求：

```markdown
> 使用 `llman sdd context --task "<描述>" --paths "<文件>"` 获取相关 specs。
> 如果命令不可用，通过 `llman sdd list --specs --json` 查看 spec 元数据。
```

原因：context 命令可能因 index 缺失而不可用，agent 需要知道降级路径。

### 3.2 triage 规则的使用方式

triage 规则在模板中以**决策块**形式嵌入，而非强制检查：

```markdown
## 变更规模判断（Triage）

在开始规划前，判断变更的性质：
- 行为合约变更（modify MUST/SHALL）→ 走完整 SDD 流程
- 实现变更（refactor/typo/test）→ 走快速路径
- 不确定 → 走完整 SDD 流程（保守选择）
```

agent 可以跳过 triage（如果用户明确要求走某条路径），但不能在没有 triage 的情况下默认走完整流程。

### 3.3 异步重建的引用方式

```markdown
> 如果 `llman sdd context` 返回 `quality: "unavailable"`：
> 1. 启动后台重建：`llman sdd index rebuild --async`（PID 显示在输出）
> 2. 用 `llman sdd list --specs --json` 继续工作
> 3. 索引就绪后 context 自动生效
```

## 4. 测试策略

### 4.1 Template rendering test

```bash
# 重新生成所有 skills 并检查是否包含新内容
llman sdd update-skills --no-interactive --all

# 检查 onboard 包含 context
grep -r "context --task" .agents/skills/llman-sdd-onboard/SKILL.md

# 检查 structured protocol 包含 triage
grep -r "判断变更规模" .agents/skills/*/SKILL.md

# 检查 quick 技能存在
ls .agents/skills/llman-sdd-quick/SKILL.md

# 检查 sdd-commands 包含 context
grep -r "llman sdd context" .agents/skills/*/SKILL.md
```

### 4.2 Agent behavior test（手动）

1. `llman sdd context --task "typo fix" --paths "src/main.rs"` → 验证 context 返回正确
2. 模拟 index 缺失场景 → 验证 async rebuild 引导出现
3. 对行为合约变更任务 → 验证 agent 选择完整 SDD 流程
4. 对实现变更任务 → 验证 agent 选择快速路径

### 4.3 Template validation test

```bash
just check-sdd-templates  # 验证模板版本一致性
```

## 5. 回退计划

| 问题 | 回退方式 |
|------|---------|
| context 命令未实现但模板已部署 | agent 看到 `quality: unavailable` 时会降级，不影响基本工作流 |
| triage 规则导致 agent 行为异常 | 回滚 `structured-protocol.md` 的约束条目，重新运行 `llman sdd update-skills` |
| quick path 被滥用 | 收紧 `quick.md` 的使用条件，或移除该技能 |
| async 重建引导不清晰 | 修改 `onboard.md` 和 `explore.md` 中的相关注记 |

## 延期实现记录

### Deferred: validate --health

`llman sdd validate --health` 的坏口味检测（僵尸 req、迷雾 spec、范围膨胀）延期到独立 change 中实现。

触发条件：
- `llman sdd context` 命令已稳定使用至少一个迭代周期
- 有明确的坏口味检测规则定义和 false positive 率评估
- 已有实验数据说明检测规则的有效性

当前假设：
- 僵尸 req：grep 关键词匹配 codebase 和 requirement statement，命中数为 0 时标记 suspected-zombie
- 迷雾 spec：requirement 缺少 scenario 或 scenario 覆盖数 < 1
- 范围膨胀：spec 的 valid_scope 覆盖目录但只有 1 个 requirement

不做的内容：
- 本 change 的 `list --json --meta` 中预留了 `health` 字段（由另一变更实现），但填充逻辑不在此实现

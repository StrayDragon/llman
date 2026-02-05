## Why
`llman sdd` 负责对 `llmanspec/` 目录树进行读写、归档与校验，属于高影响命令组。当前实现存在：
- **安全风险**：`archive` 等子命令将用户输入的 change id 直接拼接为路径，缺少“标识符校验”，理论上可能触发路径穿越并移动非预期目录。
- **交互/参数边界不够清晰**：部分 flag 组合缺少明确的冲突策略（例如 `list --specs --changes`），容易产生“看似可用但含义不明”的行为。
- **性能隐患**：批量校验时 staleness 检查为每个 spec 启动多次 git 子进程，spec 数量增多时会明显变慢。

### Current Behavior（基于现有代码）
- `archive`：`changes_dir.join(change_name)` 直接拼路径（`src/sdd/change/archive.rs`），未禁止 `../` 或分隔符。
- `list`：当前 `--specs` 优先于 `--changes`，两者同时出现时不报错（`src/sdd/shared/list.rs`）。
- staleness：批量校验中对每个 spec 调用 `evaluate_staleness(...)`（`src/sdd/shared/validate.rs`），内部多次执行 `git`（`src/sdd/spec/staleness.rs`）。

## What Changes
- 为所有接受 `change-id/spec-id` 的路径拼接点增加“标识符校验”（拒绝路径分隔符与 `..`），重点覆盖 `archive`，避免路径穿越。
- 明确并收敛 `list` 参数边界：`--specs` 与 `--changes` 互斥，冲突时显式报错。
- `update-skills`：当一次生成多个 tool 时，单个 `--path` 覆盖可能导致输出互相覆盖；默认改为显式拒绝并提示用户按 tool 分开执行（或选择安全的分目录策略）。
- staleness 性能：在批量校验中缓存共享 git 结果（base ref、merge-base、dirty、diff names），避免对每个 spec 重复起进程（保持行为一致，仅优化实现）。

### Non-Goals（边界）
- 不新增 SDD 子命令，不改变既有 spec/delta 语法格式。
- 不改变 `validate` 的严格语义（例如“缺少 delta 必须失败”仍保持为错误）；本变更聚焦安全/边界与性能实现优化。

## Impact
- Affected specs: `specs/sdd-workflow/spec.md`
- Affected code:
  - `src/sdd/shared/{list,validate}.rs`
  - `src/sdd/change/archive.rs`
  - `src/sdd/spec/staleness.rs`
  - `src/sdd/project/update_skills.rs`
- Risk/Compatibility：
  - `list` 参数冲突将从“隐式选择”变为“显式错误”（需要在变更说明中记录）。
  - `update-skills --path` 在 multi-tool 时可能从“潜在覆盖”变为“显式拒绝”（更安全，但属于行为变化）。

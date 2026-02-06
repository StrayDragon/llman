## Why
`llman skills` 当前实现对“断链 symlink（dangling symlink）”与交互取消流程处理不够健壮：
- 断链 symlink 在 `Path::exists()` 语义下会被当作“不存在”，导致覆盖/删除行为失真，重复执行可能报错或无法修复脏状态。
- 冲突提示的取消在部分路径下会被当作硬错误返回，与其它交互命令“取消=安全退出”的预期不一致。
- `registry.json` 直接覆盖写入，异常中断可能造成文件损坏，影响后续管理器行为。

### Current Behavior（基于现有代码）
- 目标链接存在性判断使用 `link_path.exists()`（`src/skills/targets/sync.rs`），断链 symlink 会返回 false。
- 冲突处理提示 `Select::prompt()` 的取消/中断会被包装为错误（`src/skills/targets/sync.rs`）。
- Registry 保存使用 `fs::write` 覆盖写（`src/skills/catalog/registry.rs`），非原子。

## What Changes
- 将断链 symlink 视为“已存在条目”来处理：覆盖/删除逻辑基于 `symlink_metadata` 而非 `exists()`。
- 交互冲突提示支持取消：用户取消应当安全跳过该项或安全退出，不产生部分变更。
- Registry 写入改为原子写（临时文件 + rename），避免中断导致 JSON 损坏。
-（可选）交互 target 选择避免 label 冲突导致误选。

### Non-Goals（边界）
- 不改变 targets/config 解析优先级与数据格式。
- 不引入新的 target mode；不改变 `--target-conflict` 的语义（仅修复边界行为）。

## Impact
- Affected specs: `specs/skills-management/spec.md`
- Affected code:
  - `src/skills/targets/sync.rs`（link create/remove 与冲突流程）
  - `src/skills/cli/command.rs`（交互选择一致性）
  - `src/skills/catalog/registry.rs`（原子保存）

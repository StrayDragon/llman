## Why
`llman tool rm-useless-dirs` 当前有两处容易踩坑的边界：
- 默认 `.gitignore` 解析基于 **当前工作目录** 而不是扫描目标目录；当用户传入 `path` 指向其它目录时，忽略规则可能完全错误。
- “protected” 保护逻辑主要按 basename 判断，缺少对路径中任意 protected component 的保护，存在在受保护子树内误遍历/误判的风险。

### Current Behavior（基于现有代码）
- `.gitignore` 默认路径：`current_dir().join(".gitignore")`（`src/tool/rm_empty_dirs.rs`），与 `args.path` 无关。
- protected target：`is_protected_target` 仅检查 `target.file_name()`（`src/tool/rm_empty_dirs.rs`），不检查路径组件。

## What Changes
- 默认 `.gitignore` 解析：当提供扫描目标 `path` 时，默认使用 `<target>/.gitignore`（存在且为文件时）。
- protected 保护增强：对扫描过程中的任意路径组件命中 protected 名称时，必须跳过遍历且不删除其内容。
-（可选）减少无谓遍历：避免在 useless dir 删除前进行昂贵的二次递归扫描（在不改变保护语义的前提下）。

### Non-Goals（边界）
- 不改变默认 protected/useless 名单内容，仅修复解析与遍历边界。
- 不改变 `--prune-ignored` 的语义（仍按现有规范）。

## Impact
- Affected specs: `specs/tool-rm-useless-dirs/spec.md`
- Affected code: `src/tool/rm_empty_dirs.rs`

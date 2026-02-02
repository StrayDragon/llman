## 1. Specification Updates
- [x] 1.1 更新 `skills-management` 规范为“先选 target 再选技能”的交互流程与默认勾选规则
- [x] 1.2 明确 registry 仅在确认应用后写入，交互默认基于 filesystem

## 2. Interactive Flow Refactor
- [x] 2.1 交互式流程改为单目标选择，再展示技能多选列表
- [x] 2.2 默认勾选基于目标目录的实际 symlink 状态
- [x] 2.3 确认后按 diff 执行 relink/remove，并仅更新该 target 的 registry 状态

## 3. Messaging and Tests
- [x] 3.1 更新交互提示文案（目标选择、技能选择、确认）
- [x] 3.2 添加/更新测试覆盖：symlink 状态判定、diff 应用、registry 写入时机

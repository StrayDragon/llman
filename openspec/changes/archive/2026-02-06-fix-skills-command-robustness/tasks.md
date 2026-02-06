## 1. Implementation（健壮性修复）
- [x] 1.1 断链 symlink 处理：使用 `symlink_metadata`/`try_exists` 判断目标项是否存在，确保断链 symlink 也能被覆盖/删除。
  - 验证：构造断链 symlink 作为冲突项，重复运行同步流程不会报 “File exists” 且能恢复为期望链接。
- [x] 1.2 冲突提示取消语义：交互冲突提示被取消时，必须整体 abort（安全退出），并保证不会写入 registry 或产生半完成状态。
  - 验证：取消冲突提示后，target 目录与 `registry.json` 均保持不变，且命令成功退出（不视为错误）。
- [x] 1.3 Registry 原子写：`registry.json` 写入采用临时文件 + rename，确保崩溃/中断不会产生半写入文件。
  - 验证：在写入失败路径（模拟）中，`registry.json` 仍是有效 JSON（旧值或新值之一）。
 - [x] 1.4（可选）交互 target 选择去歧义：避免 label 冲突或与 exit label 混淆导致误选。
   - 验证：构造重复 label 的 targets，选择结果可预测且正确。

## 2. Tests（回归与边界）
- [x] 2.1 单元测试：断链 symlink 覆盖/删除（`exists()` 为 false 但 `symlink_metadata` 可读）。
- [x] 2.2 单元测试：交互冲突提示取消会整体 abort 且不产生任何写入（可通过注入/模拟 prompt 层实现）。
- [x] 2.3 单元测试：registry 原子写（验证写入结果永远是有效 JSON）。

## 3. Acceptance（验收标准）
- [x] 3.1 所有新增规范场景可通过测试验证。
- [x] 3.2 重复运行 `llman skills`（交互/非交互）在断链 symlink 场景下具备幂等性（无报错、无不必要变更）。
- [x] 3.3 registry 永不出现损坏 JSON。
- [x] 3.4 交互取消具备“整体 no-op”语义（不产生任何变更且不视为错误）。

## 4. Validation
- [x] 4.1 `openspec validate fix-skills-command-robustness --strict --no-interactive`
- [x] 4.2 `just test`

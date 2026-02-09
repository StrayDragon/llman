---
name: "llman-sdd-verify"
description: "验证实现是否与 llman SDD 的 specs/design 一致，并给出最小修复建议。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD Verify

使用此 skill 验证实现是否与该 change 的 artifacts 一致。

## 步骤
1. 确定 change id（不明确时让用户从 `llman sdd list --json` 选择）。
2. 阅读：
   - `llmanspec/changes/<id>/specs/` 下的 delta specs
   - `proposal.md` 与 `design.md`（如存在）
   - `tasks.md`（理解实现范围）
3. 对比 artifacts 与代码：
   - 标出不一致（缺失行为、错误行为、缺测试/文档）
   - 给出最小修复建议或建议更新 artifacts
4. 运行仓库的验证命令（tests/lint 等，视项目而定）。
5. 若一致且验证通过，建议归档：`llman sdd archive <id>`。

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

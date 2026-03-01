---
name: "llman-sdd-verify"
description: "验证实现是否与 llman SDD 的 specs/design 一致，并给出最小修复建议。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD Verify

使用此 skill 验证实现是否与该 change 的 artifacts 一致。

## 步骤
1. 确定 change id（不明确时让用户从 `llman sdd-legacy list --json` 选择）。
2. 先跑一个快速校验门禁：
   - `llman sdd-legacy validate <id> --strict --no-interactive`
3. 阅读：
   - `llmanspec/changes/<id>/specs/` 下的 delta specs
   - `proposal.md` 与 `design.md`（如存在）
   - `tasks.md`（理解实现范围）
4. 对比 artifacts 与代码：
   - 标出不一致（缺失行为、错误行为、缺测试/文档）
   - 给出最小修复建议或建议更新 artifacts
5. 输出简短报告：
   - **CRITICAL**（归档前必须修复）
   - **WARNING**（建议修复）
   - **SUGGESTION**（可选优化）
6. 若存在 CRITICAL，建议用 `llman-sdd-apply`（或 `/llman-sdd:apply <id>`）修复；若通过则建议归档：`llman sdd-legacy archive <id>`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

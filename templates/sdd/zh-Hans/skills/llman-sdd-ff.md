---
name: "llman-sdd-ff"
description: "Fast-forward：一次性创建 proposal/specs/design/tasks。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD Fast-Forward (FF)

使用此 skill 快速为一个新 change 创建 **全部** artifacts（proposal → specs → design（可选）→ tasks）。

## 步骤
1. 询问用户：
   - 变更的一句话描述
   - 期望的 change id（或你来派生；kebab-case + 动词前缀）
   - 受影响的 capability（用于创建 `specs/<capability>/`）
2. 如果 `llmanspec/changes/<id>/` 已存在，STOP 并建议使用 `llman-sdd-continue`。
3. 在 `llmanspec/changes/<id>/` 下创建 artifacts：
   - `proposal.md`
   - `specs/<capability>/spec.md`（至少一个）
   - `design.md`（仅当需要时）
   - `tasks.md`（有序、小步、可验证，包含校验步骤）
4. 校验：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```
5. 给出简短状态总结，并建议下一步（`llman-sdd-apply` 或 `/opsx:apply`）。

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}
{{region: templates/sdd/zh-Hans/skills/shared.md#validation-hints}}

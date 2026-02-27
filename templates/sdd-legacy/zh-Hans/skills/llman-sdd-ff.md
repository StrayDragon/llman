---
name: "llman-sdd-ff"
description: "Fast-forward：一次性创建 proposal/specs/design/tasks。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD Fast-Forward (FF)

使用此 skill 快速为一个新 change 创建 **全部** artifacts（proposal → specs → design（可选）→ tasks）。

## 步骤
1. 询问用户：
   - 变更的一句话描述
   - 期望的 change id（或你来派生；kebab-case + 动词前缀）
   - 受影响的 capability（用于创建 `specs/<capability>/`）
   - 在创建任何目录前，先让用户确认最终 id。
2. 确保项目已初始化：
   - 必须存在 `llmanspec/`；若不存在，提示先运行 `llman sdd init`，然后 STOP。
3. 如果 `llmanspec/changes/<id>/` 已存在，询问用户是否：
   - 继续补齐缺失工件（推荐），或
   - 改用其他 id。
   不要在未明确确认的情况下覆盖已有工件。
4. 在 `llmanspec/changes/<id>/` 下创建 artifacts：
   - `proposal.md`
   - `specs/<capability>/spec.md`（至少一个）
   - `design.md`（仅当需要时）
   - `tasks.md`（有序、小步、可验证，包含校验步骤）
5. 校验：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```
6. 给出简短状态总结，并建议下一步（`llman-sdd-apply` 或 `/llman-sdd:apply`）。

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}

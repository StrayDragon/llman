---
name: "llman-sdd-show"
description: "快速查看 llmanspec 变更与 specs。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD 查看

使用此 skill 快速查看变更与 specs。

## 步骤
1. 列出条目：`llman sdd list` 或 `llman sdd list --specs`。
2. 如果 id 不明确，展示列表并让用户选择。
3. 查看详情：`llman sdd show <id>`。
4. 需要时使用 `--type change|spec` 消除歧义。
5. 对 change 使用 `--json`：状态 SSOT 字段为 `stage` / `specsLanded` / `skipSpecsLanding` / `readyToImplement`（勿凭「完整工件」口头判断可否 apply）。

> 命令细节用 `llman sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表（r139）。

{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}

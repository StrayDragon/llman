---
name: "llman-sdd-sync"
description: "手动把 delta specs 同步到主 specs（不归档 change）。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Sync

使用此 skill 将活动 change 的 delta specs 同步到主 specs（**不归档** change）。

这是一个手动、可复现的协议。

## 步骤
1. 确定 change id（不明确时让用户选择）。
   - 始终说明："使用变更：<id>"。
2. 模式检查（`llmanspec/config.yaml`）：
   - **BDD-on（Git-native）**：无需 sync——feature 分支上的 live `llmanspec/specs/**` 即 SSOT。用 `llman sdd change diff <id>` 只读审查。**不要**编造 `feature_delta` apply。准备好后：优先 `change finalize`（单 commit）或 fallback `checkpoint` → `change archive`（仅文档）→ Git/PR merge。
   - **BDD-off**：对每个 delta capability，手动将 `changes/<id>/specs/<capability>/spec.toon` → 主 `specs/<capability>/spec.toon`（经典 TOON delta 合并）。无 harness/分支要求。
3. 校验 specs：
   ```bash
   llman sdd validate --specs --strict --no-interactive
   ```
4. sync 不负责归档；准备好后执行 `llman sdd change finalize <id>`（BDD-on 推荐）或 `llman sdd change archive <id>`。

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}

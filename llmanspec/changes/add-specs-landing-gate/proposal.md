---
depends_on: []
---

# Specs landing gate：合约只在绑定分支落地

## Why

Git-native 流程废除 `change/specs/` delta 后，live `llmanspec/specs/**` 成了公共 SSOT，但流水线与 propose skill 未写明「该 change 的 specs 何时、在哪条分支落地」。Agent/用户常在默认分支先改 specs 再 `change start`，与干净树门禁打架，结果把未实现合约 commit 进 main，或多 change 抢同一 HEAD。

今日 `readyToImplement` 在仅有 attach binding（Full）时即为 true，CLI **不**检查 `base...branch` 是否含 `llmanspec/specs/**` diff，apply 可能在无合约增量时启动。

## What Changes

- 固化 **Specs landing** 概念：仅在 change 绑定的非默认分支上编辑/提交 `llmanspec/specs/**`；合入默认分支的窗口是 archive/finalize 的 ff-merge。
- 纠正 propose skill / 根 AGENTS 顺序：**先** `change start`（或 attach），**再**改 live specs；禁止为过干净树把门禁把 live specs commit 到默认分支。
- CLI：`show`/`status` 暴露 `specsLanded`；`readyToImplement` 收紧为 Full ∧ (specsLanded ∨ `skip_specs_landing`)；`validate` 对未落地给 WARNING；错误文案引导 agent 用正确 skill（勿重复 `start`）。
- Frontmatter 增加可选豁免字段 `skip_specs_landing`（无 live 合约变更的 change）。

## Capabilities

- `sdd-workflow`（stage / readyToImplement / specs landing / start 文案澄清）
- 托管 skill 模板与根 `AGENTS.md`（流程说明）

## Impact

- 破坏性：既有「仅 attach 即 readyToImplement=true」的 BDD/调用方需适配；无 specs diff 的 Full change 将不再 ready，除非 `skip_specs_landing: true`。
- Skills：`llman-sdd-propose` / `apply` / 根 AGENTS 文案同步；`init --update` 后用户侧 skill 刷新。

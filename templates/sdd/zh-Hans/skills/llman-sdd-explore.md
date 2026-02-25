---
name: "llman-sdd-explore"
description: "进入 llman SDD 探索模式（仅思考；不做实现）。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD Explore

当用户希望在开始实现之前先理清思路、调查问题或澄清需求时，使用此 skill。

**重要：探索模式只用于思考，不用于实现。**
- 你可以阅读文件、搜索代码、调查代码库。
- 如果用户需要，你可以创建/更新 llman SDD artifacts（proposal/specs/design/tasks）。
- 你绝对不能在探索模式下写应用代码或实现功能。

## 探索姿态（Stance）
- 好奇而不教条
- 以真实代码为依据
- 需要时用 ASCII 图可视化
- 同时保留多个选项与权衡

## 建议动作
1. 先看上下文：`llman sdd list --json`
2. 如果某个 change id 相关，阅读 `llmanspec/changes/<id>/` 下的 artifacts。
3. 提出 1-3 个澄清问题，然后讨论选项与权衡。
4. 当结论逐渐清晰时，建议用户把它记录下来（不要自动写入）：
   - 范围变化 → `proposal.md`
   - 需求变化 → `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - 设计决策 → `design.md`
   - 新工作项 → `tasks.md`

## 退出探索模式
当用户准备开始实现时，建议：
- `/opsx:new` 或 `llman-sdd-new-change`（创建 change）
- `/opsx:ff` 或 `llman-sdd-ff`（一次性创建所有 artifacts）
- `llman-sdd-apply`（按 tasks 实施）

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#structured-protocol}}
{{region: templates/sdd/zh-Hans/skills/shared.md#future-planning}}

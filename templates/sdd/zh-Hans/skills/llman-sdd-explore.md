---
name: "llman-sdd-explore"
description: "进入 llman SDD 探索模式：理清思路、调查需求、分析问题。仅思考，禁止写代码。用于意图不明确或需要分析后再行动的场景。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Explore

当用户希望在开始实现之前先理清思路、调查问题或澄清需求时，使用此 skill。

**重要：探索模式只用于思考，不用于实现。**
- 你可以阅读文件、搜索代码、调查代码库。
- 如果用户需要，你可以创建/更新 llman SDD artifacts（proposal/specs/design/tasks）。
- 你绝对不能在探索模式下写应用代码或实现功能。

## Pipeline 位置

```mermaid
flowchart LR
    explore["★ llman-sdd-explore ★<br/>探索（你现在在这里）"]
    explore --> propose["llman-sdd-propose<br/>提案"]
    propose --> apply["llman-sdd-apply<br/>实施"]
    apply --> verify["llman-sdd-verify<br/>验证"]
    verify --> archive["llman-sdd-archive<br/>归档"]
    archive --> commit["git commit<br/>完成闭环"]

    style explore fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 你现在在探索阶段（仅思考）→ 常规路径下一步 `llman-sdd-propose`（提案）
> 📎 如果是小改动（不改行为合约），可直接走 `llman-sdd-quick`（快速路径）

## 探索姿态
- 好奇而不教条
- 以真实代码为依据
- 需要时用 ASCII 图可视化
- 同时保留多个选项与权衡

## 建议动作
1. 使用 `llman sdd context --task "<任务>" --paths "<文件>"` 快速定位相关 specs。
   - 阅读 context 的 `direct` 列出的 spec 全文（这些是必须理解的合约）。
   - 如果 context 不可用，运行 `llman sdd index rebuild`（默认 `pageindex`，无需模型）后重试。
2. 澄清目标与约束（问 1–3 个问题）。
3. 如果某个 change id 相关，阅读 `llmanspec/changes/<id>/` 下的 artifacts。
   - 诊断校验错误时优先跑 `llman sdd validate <spec> --strict --no-check`（fast mode，跳过可能耗时的 `bdd.run_command`），先解决结构门禁（Gherkin / `@req` 链接 / 双写 / req_id 唯一性），再跑 full mode（`--check` 或 `cargo test --features bdd`）。错误输出中的 `FAIL <item_type>/<id>` 行会逐条指明失败项。
4. 探索 2–3 个选项与权衡。
5. 判断变更规模（triage），确定是否需要走完整 SDD 流程。
6. 当结论逐渐清晰时，建议用户把它记录下来（不要自动写入）：
   - 范围变化 → `proposal.md`
   - BDD-off 约束/场景 → `llmanspec/changes/<id>/specs/<capability>/spec.toon`（TOON delta）
   - BDD-on 约束 → feature 分支上的 live `llmanspec/specs/<capability>/spec.toon`
   - BDD-on 可执行 harness → live `llmanspec/specs/<capability>/*.feature`（`@req`）；禁止 `*.feature.delta.toon`
   - 设计决策 → `design.md`
   - 新工作项 → `tasks.md`

> BDD-on（Git-native Partitioned）：feature 分支 + live `.feature`/`spec.toon` 为 SSOT；用 `change attach` 绑定；无 solidify / feature_delta。

## 退出探索模式
当用户准备开始实现时，根据变更规模选择路径：
- 行为合约变更 → `llman-sdd-propose`（创建提案工件）
- 小改动 / 不改合约 → `llman-sdd-quick`（快速路径）
- 已有完整 change 工件 → `llman-sdd-apply`（按 tasks 实施）
若用户在探索模式中要求你开始实现，STOP 并提醒其先退出探索模式。

> 💡 探索完成 → 下一步 `llman-sdd-propose`（保单）或 `llman-sdd-quick`（快速路径）

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}

# language: zh-CN
# capability: sdd-ab-evaluation
# purpose: 规范可复现的 SDD prompts 与 agentic 工作流评测套件，含多风格对比与硬门禁校验。
# scope: llmanspec/specs/sdd-ab-evaluation

功能: sdd-ab-evaluation

  @req:r25 @human
  场景: multi-style 对比、快照、聚合与可选软评分
    - The system MUST satisfy the harness scenarios for `multi-style 对比、快照、聚合与可选软评分`: 对应 spec: sdd-ab-evaluation — 评测输出优先安全与质量信号；评测套件含需读编辑格式相关 spec 文件的 agentic 任务；支持 multi-style 对比；baseline 预置语义等价；每次 run 产可观测快照； --runs N 产聚合报告；可选软评分层；docker runner 支持阿里云镜像。

  @req:r56 @human
  场景: 可复现的 SDD prompts 评测套件与 Claude Code agentic 评测
    - 对应 spec: sdd-ab-evaluation — SDD workflow MUST 提供可复现的 Promptfoo 评测套件对比不同风格/

  @req:r118 @human
  场景: skill 模板评测基线: skill-in-prompt × sandbox 硬门禁组合
    - SDD 评测套件 MUST 提供一种组合评测能力（对应 fixture `sdd_skill_gate_v1`）：把指定 skill 模板渲染进 agentic prompt（`llman sdd init --update` 产物经 strip frontmatter 后注入 agent prompt），同时在隔离 sandbox（独立 `LLMAN_CONFIG_DIR` + 临时 init 的 llmanspec 项目 + git baseline + 本地 llman binary on PATH）内驱动 agent 执行真实任务，并以 `llman sdd validate --all --strict` 作为确定性硬门禁断言（pass/fail）。该套件 MUST 支持真正的跨模板版本 A/B 对比：`--baseline-skill <path>` 接受上一版模板快照、`--candidate-skill` 默认当前工作区模板，两者 MUST 经由 per-provider prompt override（baseline provider 用 baseline skill 渲染的 prompt、candidate provider 用 candidate skill 渲染的 prompt）在一次 eval run 中并行评测——MUST NOT 让两 provider 共享同一 prompt 导致 A/B 退化为重复测量；评分信号 MUST 至少含硬门禁通过率与成本指标（token/turns，来自 promptfoo 原生 metrics）。`--runs N` (N>=2) 时 MUST 产按 baseline/candidate 维度的聚合报告（pass_rate + token/turns/cost 的 mean/median/p90）。该套件 MUST 覆盖 apply、draft、propose 三类 skill；MAY 不含 LLM-rubric 软分与 golden 参考锚点（留待后续）。该套件 MUST NOT 改动既有 `sdd_apply_v1` 与 `sdd_llmanspec_styles_v1/v2` suite（向后兼容）。

  @req:r120 @human
  场景: skill-gate 持久化基线存储
    - skill-gate 评测套件 MUST 提供 `agentdev/promptfoo/baselines/` 目录用于版本化存储历史 SKILL.md 快照（经 `git show <ref>:templates/.../SKILL.md` 产生），runner 的 `--baseline-skill` 默认指向该目录下的快照文件。baselines/ MUST 纯人工/CI 管理（eval 运行 MUST NOT 自动写入 baselines/，避免副作用污染）。baselines/ MUST 含 README 说明快照产生方式。

  @req:r119 @human
  场景: 按 skill 定制 sandbox 预置
    - skill-gate 评测套件 MUST 按 `--skill` 参数定制 sandbox 的起始状态与对应 task_prompt：apply 预置带 tasks.md 的 change shell、draft 预置空项目（任务是用 `change new --from` 记草案）、propose 预置空 capability spec skeleton（任务是走完整 propose）。runner MUST 从 tests.yaml 中按 skill 字段选取对应的 task_prompt，而非固定取第一个。

---
depends_on: []
---

纯流程 / 模板优化，目标是不写应用代码就把「人类如何高效 review `.feature` 固化下来」。
已确认排在 `sdd-review-aggregate-html-view`（命令化形态）之前先行。

## Why

当前人类 review 缺少固定入口约定：unbound scenarios、pending `@req`、staleness、
locked-rule diff 散落在 `list --specs` / `show` / `validate` / `graph` 各处，
每次 review 都靠临场拼装命令组合；verify 双轴报告也没有明确的人读落点。
agents 需要 AGENTS.md 给出统一 checkpoint 指引，人才有稳定的协同节奏。

## What Changes

- AGENTS.md SDD 段新增「Human Review Checkpoint」小节：定义何时进入人审
  （apply 前 / verify 后 / archive 前）、推荐命令序列与预期输出解读、以及发现分歧时
  回到 explore/propose 的升级路径。
- templates/sdd 技能模板补强：propose / verify 模板增加面向人类读者的摘要段落要求
  （让 verify 报告首屏即是人读结论而非机器明细）。
- 所有模板改动 en / zh-Hans 双语 parity，过 `just check-sdd-templates`。

## Non-goals

- 不新增 CLI 子命令（那是下一个 change 的事）。
- 不改 live `.feature` 行为合约文本；如触发 sdd-structured-skill-prompts 能力边界的
  MUST 条款，则升级走 propose。

## Open Questions

- Human Review Checkpoint 写进 AGENTS.md 自由区还是 managed block？托管块会随
  `init --update` 刷新，需确认不会覆盖手写内容。
- verify 模板的「人读摘要」格式要不要固化字段集（结论/风险/待决策），还是仅原则性要求？

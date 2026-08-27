# Design — feat-sdd-review-workflow-suite

## 已决事项（用户拍板）

| # | 议题 | 决议 |
|---|------|------|
| D-A | `--export-html` 模板位置 | **canonical 授权地 = `templates/sdd/`**（可进 parity/lint 检查管线）；发布形态经编译期 `include_str!` 内嵌，保证二进制自包含、端侧零运行时依赖 |
| D-B | 配置面 | **v1 零配置**：不新增 config.yaml 字段；五类信号默认全开；阈值/排除项推迟到真实需求出现 |
| D-C | 锁定规则 diff 粒度 | v1 仅提示「有变化」+ 指引 `change diff` 查看对照行（避免 review 输出爆炸；对照渲染归 finalize/checkpoint 门禁已有输出） |
| D-D | Human Review Checkpoint | 写入 root AGENTS.md **自由区**（Testing Guidelines 同级新小节），不入 LLMAN 托管块 |

Seam 确认（r101，用户已批）：**S1** `llman sdd review` CLI 子进程；**S2**
`--export-html` 文件产物边界；**S4** review↔list/show 数据互恰（挂既有 req 口径）。
T0 形状冻结不加新 seam，以既有 r134 场景 + 快照单测作证明层。

## 新 capability：`sdd-review`

`llmanspec/specs/sdd-review/sdd-review.feature` 承载：
- r5 起（next-req-id 全局分配）逐条 @human 规则：聚合信号覆盖、CRITICAL 退出码策略、
  --json 同构字段、--export-html 单文件离线契约、零配置约束、展示层冻结引用（指向
  spec-format r133/r134 与 sdd-workflow r39 的口径，不改其文本）。
- @executable 子场景挂回上述规则，全部复用既有泛型 Gherkin 步骤库动词
  （运行/退出码/stdout 包含/相对路径存在/内容包含），保证 rstest-bdd 绑定零新增。

## 风险与缓解

- **templates/sdd 管线兼容**：check-sdd-templates.py 目前扫描 en/zh-Hans 目录与版本头；
  新增共享 UI 模板放置于 `templates/sdd/shared/`（脚本未扫区域，apply 时核实脚本的
  glob 行为并按需加白名单断言——「文件存在且含 <html> 收尾」级轻校验，不动 parity 机制）。
- **聚合计数漂移**：review 必须以同一 parser/morphology 后端取数（复用 compute_rule_morphology
  与 staleness 模块），禁止第二套统计实现——由 S4 互恰 executable 守护。
- **HTML 注入面**：capability/statement 文本进 HTML 前做最小转义（单测覆盖 `<` 场景）。

## expand-contract 说明

T0 深化为纯内部重构（行为冻结证明先行），随后 T1-T3 为纯增量；无旧路径退役需求。

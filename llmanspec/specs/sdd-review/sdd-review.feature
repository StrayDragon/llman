# language: zh-CN
# capability: sdd-review
# purpose: 定义 llman sdd review 人审聚合命令：五类信号聚合、零配置、退出码策略、JSON 同构与单文件离线 HTML 视图。
# scope: src/sdd

功能: sdd-review

  @req:r5 @human
  场景: review 聚合命令与信号覆盖
    - 系统 MUST 提供 `llman sdd review` 命令，单次运行聚合以下信号并逐项标注来源口径：pending 与 manual 约束规则（r134 键名）、harness unbound 验收场景、staleness 提示、绑定分支锁定规则 diff 概要（仅提示有变化并指引 change diff，见 D-C）、`validate --all --strict --no-check` 的 FAIL/WARNING 清单。--capability <id> MUST 作为唯一作用域过滤参数。

  @req:r6 @human
  场景: 零配置契约
    - v1 MUST NOT 引入任何 config.yaml 新字段；上述信号默认全部启用；无项目 config 时 MUST 以明确错误退出而非静默空结果。

  @req:r20 @human
  场景: 退出码策略
    - 存在 CRITICAL 级发现（validate ERROR、悬空 @req 等）MUST 以非零退出码结束；仅 WARNING/pending 类发现 MUST 退出零。退出码策略 MUST 供 CI 与 agent 门禁直接复用。

  @req:r38 @human
  场景: review JSON 同构
    - review --json MUST 输出与文本同构的结构：signals 数组（kind/capability/count/detail）与 summary（criticalCount/warningCount）；所有计数值 MUST 与 list --specs（r39 形态）及 show morphology（r134 键名）同源同值，MUST NOT 出现第二套统计实现。

  @req:r51 @human
  场景: 单文件离线 HTML 视图
    - review --export-html <path> MUST 产出单个自包含 HTML 文件：内嵌 mermaid capability↔req↔scenario 层级图与过滤器；MUST NOT 引用外部网络资源或要求本地 server；动态文本 MUST 经最小转义。模板 canonical 位于 templates/sdd/shared/ 并在编译期内嵌进二进制。

  @executable
  @req:r5
  场景: review-default-run-lists-signals
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd review
    那么 退出码为零
    那么 stdout 包含 pending
    那么 stdout 包含 unbound
    那么 stdout 包含 stale

  @executable
  @req:r20
  场景: review-exit-zero-without-critical
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd review
    那么 退出码为零

  @executable
  @req:r20
  场景: review-exit-nonzero-on-validate-error
    假如 已初始化含损坏 proposal 的 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd review
    那么 退出码非零

  @executable
  @req:r38
  场景: review-json-shape
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd review --json
    那么 退出码为零
    那么 stdout 为合法 JSON 且含 JSON 键 summary
    那么 stdout 的 JSON 键 summary.criticalCount 为数字

  @executable
  @req:r51
  场景: review-html-artifact-selfcontained
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd review --export-html review.html
    那么 退出码为零
    那么 相对路径 review.html 存在
    那么 相对路径 review.html 内容包含 mermaid

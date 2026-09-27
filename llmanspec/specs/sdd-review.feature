# language: zh-CN
# capability: sdd-review
# purpose: 定义 llman sdd review 人审聚合命令：四类信号聚合（pending/stale/locked/validate）、零配置、退出码策略、JSON 同构与单文件离线 HTML 视图。
# scope: src/external_command.rs

功能: sdd-review

  @req:r5
  规则: review 聚合命令与信号覆盖
    系统 MUST 提供 `llman sdd review` 命令，单次运行聚合以下信号并逐项标注来源口径：pending（裸规则计数，见 spec-format r134）、staleness 提示、locked（0.5 恒为 0，见 spec-format r135）、`validate --all --strict --no-check` sweep 的 FAIL/WARNING 清单。harness unbound 验收场景与 manual 约束口径 MUST NOT 再出现（0.5 信号集收窄为 pending/stale/locked/validate）。--capability <id> MUST 作为唯一作用域过滤参数，仅限定 pending 与 stale 两类信号（locked 与 validate sweep 保持全局口径）。

    场景: review-default-run-lists-signals
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd review
      那么 退出码为零
      那么 stdout 包含 pending
      那么 stdout 不含 unbound
      那么 stdout 包含 stale
  @req:r6
  规则: 零配置契约
    v1 MUST NOT 引入任何 config.yaml 新字段；上述信号默认全部启用；无项目 config 时 MUST 以明确错误退出而非静默空结果。
  @req:r20
  规则: 退出码策略
    存在 CRITICAL 级发现（validate sweep 的 FAIL 清单：结构门 ERROR、规则缺 @req、全局重复 req_id 等，见 spec-format r132）MUST 以非零退出码结束；仅 WARNING/pending 类发现 MUST 退出零。退出码策略 MUST 供 CI 与 agent 门禁直接复用。

    场景: review-exit-zero-without-critical
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd review
      那么 退出码为零

    场景: review-exit-nonzero-on-validate-error
      假如 已初始化含损坏 proposal 的 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd review
      那么 退出码非零
  @req:r38
  规则: review JSON 同构
    review --json MUST 输出与文本同构的结构：signals 数组（kind/capability/count/detail）与 summary（criticalCount/warningCount）；warningCount MUST 等于 pending 与 stale 两类信号计数之和，criticalCount MUST 等于 sweep FAIL 涉及的 capability 数；所有计数值 MUST 与 list --specs（r39 形态）及 show morphology（spec-format r134 原生五键）同源同值，MUST NOT 出现第二套统计实现。

    场景: review-json-shape
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在非交互终端运行 llman sdd review --json
      那么 退出码为零
      那么 stdout 为合法 JSON 且含 JSON 键 summary
      那么 stdout 的 JSON 键 summary.criticalCount 为数字
  @req:r51
  规则: 单文件离线 HTML 视图
    review --export-html <path> MUST 产出单个自包含 HTML 文件：内嵌 mermaid capability↔req↔scenario 层级图与过滤器；MUST NOT 引用外部网络资源或要求本地 server；动态文本 MUST 经最小转义。报告模板由外部 llman-sdd 二进制内嵌提供，本仓库不持有模板产物。

    场景: review-html-artifact-selfcontained
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 运行 llman sdd review --export-html review.html
      那么 退出码为零
      那么 相对路径 review.html 存在
      那么 相对路径 review.html 内容包含 mermaid

# language: zh-CN
# 对应 spec: sdd-bdd-mode-compat r2 — solidify 随 BDD 模式切换（Partitioned：一致性门禁）。
# BDD-on 时执行 consistency 检查；BDD-off 时 no-op 并提示未配置。
功能: solidify 的模式开关
  背景:
    假如 llman 二进制已构建

  @req:r57
  场景: BDD-on 时 solidify 产出 .feature 文件
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd solidify add-scen
    那么 stdout 包含 consistency

  @req:r57
  场景: BDD-off 时 solidify 为 no-op 并提示未配置
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd solidify add-scen
    那么 退出码为零
    那么 stdout 包含 not configured

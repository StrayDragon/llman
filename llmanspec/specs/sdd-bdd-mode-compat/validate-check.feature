# language: zh-CN
# 对应 spec: sdd-bdd-mode-compat r1 — validate 的 --check/--no-check 行为随 BDD 模式切换。
# BDD-on 时 validate 默认（或 --check）执行 BDD runner；--no-check 跳过。
# BDD-off 时 --check 不执行 runner，仅降级为 INFO 提示。
功能: validate 的 check 语义随 BDD 模式切换
  背景:
    假如 llman 二进制已构建

  @req:r1
  场景: BDD-on 时 validate 默认执行 BDD runner
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd validate sample --strict
    那么 stderr 包含 BDD check failed

  场景: BDD-on 时 validate --no-check 跳过 runner
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码为零
    那么 stderr 不含 BDD check failed

  场景: BDD-off 时 validate --check 不执行 runner
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd validate sample --strict --check --json
    那么 退出码为零
    那么 stdout 为合法 JSON
    那么 stderr 不含 BDD check failed

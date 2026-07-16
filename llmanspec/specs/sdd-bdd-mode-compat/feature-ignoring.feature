# language: zh-CN
# 对应 spec: sdd-bdd-mode-compat r4 — BDD-off 项目静默忽略 .feature 文件。
# 即使 spec 目录旁有一个格式错误的 .feature，validate 也不解析它、不报 Gherkin 错。
功能: BDD-off 时 validate 静默忽略 feature 文件
  背景:
    假如 llman 二进制已构建

  场景: BDD-off 时 validate 忽略格式错误的 feature 文件
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码为零
    那么 stderr 不含 Gherkin

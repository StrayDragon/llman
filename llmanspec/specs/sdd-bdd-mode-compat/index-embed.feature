# language: zh-CN
# 对应 spec: sdd-bdd-mode-compat r78 — index rebuild 的 .feature embed 行为。
# BDD-on 项目（含 .feature）rebuild 后 tree.json 编入 spec-level scenario；
# BDD-off 项目（无 .feature）不产生 spec-level scenario。
功能: index rebuild 的 feature embed 随 BDD 模式切换
  背景:
    假如 llman 二进制已构建

  @executable
  场景: BDD-on 时 index rebuild 成功
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd index rebuild
    那么 stdout 包含 rebuilt

  @executable
  场景: BDD-off 时 index rebuild 成功且无 feature embed
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd index rebuild
    那么 stdout 包含 rebuilt

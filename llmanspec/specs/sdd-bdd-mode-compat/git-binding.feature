# language: zh-CN
# 对应 spec: sdd-bdd-mode-compat r57 — Git-native change binding + change 生命周期命令面。
# BDD-on：attach 绑定非默认分支；change new 可用；change delta 拒绝；solidify 不存在。
# BDD-off：attach 失败并提示需 bdd。
功能: Git-native change binding 的模式开关
  背景:
    假如 llman 二进制已构建

  @executable @req:r57
  场景: BDD-on 时 attach 拒绝默认分支
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change attach add-scen
    那么 退出码非零
    那么 stderr 包含 default branch

  @executable @req:r57
  场景: BDD-off 时 change attach 失败并提示需 bdd
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd change attach add-scen
    那么 退出码非零
    那么 stderr 包含 BDD-on

  @executable @req:r57
  场景: solidify 子命令不存在
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd solidify add-scen
    那么 退出码非零

  @executable @req:r57
  场景: change new 创建 proposal 草稿
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change new add-cli-new
    那么 退出码为零
    那么 stdout 包含 proposal.md

  @executable @req:r57
  场景: BDD-on 时 change delta 被拒绝
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change delta skeleton add-scen sample
    那么 退出码非零
    那么 stderr 包含 BDD-off only

  @executable @req:r57
  场景: change checkpoint 接受 --no-interactive flag
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change checkpoint add-scen --no-interactive
    那么 stderr 不含 unexpected argument

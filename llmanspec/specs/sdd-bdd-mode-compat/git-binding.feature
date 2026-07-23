# language: zh-CN
# 对应 spec: sdd-bdd-mode-compat r57 + sdd-workflow r111/r115 — 统一 Git-native
# change binding 与生命周期命令面。统一流程下不再按 bdd 段分叉 attach/delta；
# change start 为推荐入口（自动建分支 + clean-tree 门禁），attach 为手动共存命令；
# change delta 在任何模式下都被拒绝（已移除）；solidify 不存在。
功能: 统一 Git-native change binding 的命令面
  背景:
    假如 llman 二进制已构建

  @executable @req:r57
  场景: 默认分支上 change attach 拒绝
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change attach add-scen
    那么 退出码非零
    那么 stderr 包含 default branch

  @executable @req:r57
  场景: 无 bdd 段时 change attach 仍可用（统一流程）
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    而且 变更 add-scen 含 proposal design tasks 且 attach 状态为 "no"
    当 在非交互终端运行 llman sdd change attach add-scen
    那么 stderr 不含 BDD-on

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
  场景: change delta 在任何模式下都被拒绝（统一流程）
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change delta skeleton add-scen sample
    那么 退出码非零
    那么 stderr 包含 removed

  @executable @req:r57
  场景: 无 bdd 段时 change delta 同样被拒绝
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd change delta skeleton add-scen sample
    那么 退出码非零
    那么 stderr 包含 removed

  @executable @req:r57
  场景: change checkpoint 接受 --no-interactive flag
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change checkpoint add-scen --no-interactive
    那么 stderr 不含 unexpected argument

  @executable @req:r57
  场景: change start 接受 --no-interactive flag
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change start add-scen --no-interactive
    那么 stderr 不含 unexpected argument

  @executable @req:r94
  场景: finalize 接受 --no-interactive flag
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change finalize add-scen --no-interactive
    那么 stderr 不含 unexpected argument

  @executable @req:r94
  场景: 无 bdd 段时 change finalize 仍可用（统一流程）
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd change finalize add-scen
    那么 stderr 不含 BDD-on

  @executable @req:r57
  场景: change start 在干净工作区成功创建分支
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd change start add-scen
    那么 退出码为零
    那么 stdout 包含 started

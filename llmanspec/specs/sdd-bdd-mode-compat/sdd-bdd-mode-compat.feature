# language: zh-CN
# managed by llman sdd (single-track feature-as-spec)
功能: sdd-bdd-mode-compat

  @executable @req:r83
  场景: BDD-off 时 validate 静默忽略 .feature 文件
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码为零

  @executable @req:r78
  场景: BDD-on 时 index rebuild 编入 feature 派生的 scenario
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd index rebuild
    那么 stdout 包含 rebuilt

  @executable @req:r85
  场景: migrate --kind partitioned 已移除
    假如 已初始化 sdd 项目且 bdd 配置为 "on"
    当 在非交互终端运行 llman sdd project migrate --kind partitioned --dry-run
    那么 退出码非零
    那么 stderr 包含 toon2features

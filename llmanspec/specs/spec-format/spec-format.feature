# language: zh-CN
功能: 单轨 feature-as-spec 格式

  @executable @req:r136
  场景: migrate-toon2features-converts-and-cleans
    假如 已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd project migrate --kind toon2features --yes
    那么 退出码为零
    那么 相对路径 llmanspec/specs/sample/spec.toon 不存在
    那么 相对路径 llmanspec/specs/sample/sample.feature 存在

  @executable @req:r136
  场景: migrate-spec-md2toon-retired
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd project migrate --kind spec-md2toon
    那么 退出码非零
    那么 stderr 包含 toon2features

  @executable @req:r131
  场景: legacy-spec-toon-fails-with-pointer
    假如 已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "off"
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码非零
    那么 stderr 包含 toon2features

  @executable @req:r132
  场景: dangling-req-link-fails-strict
    假如 已初始化含无效 @req 的 sdd 项目且 bdd 配置为 on
    当 在非交互终端运行 llman sdd validate sample --strict --no-check
    那么 退出码非零
    那么 stderr 包含 @req

  @executable @req:r133
  场景: scaffold-emits-single-track-skeleton
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd spec skeleton demo-cap --force
    那么 退出码为零
    那么 相对路径 llmanspec/specs/demo-cap/demo-cap.feature 存在

  @executable @req:r134
  场景: list-specs-reports-rule-tier-counts
    假如 已初始化 sdd 项目且 bdd 配置为 "off"
    当 运行 llman sdd list --specs
    那么 退出码为零
    那么 stdout 包含 enforced

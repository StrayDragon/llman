# language: zh-CN
# capability: config-schemas
# purpose: 规范 llman 配置 JSON schema 的生成与校验行为。
# scope: llmanspec/specs/config-schemas.feature

功能: config-schemas

  @req:r18
  规则: schema header 经 root discovery 应用且最小侵入
    对应 spec: config-schemas — llman self schema apply MUST 通过 root discovery 定位 project/ llmanspec 配置（而非假设 cwd 为根）；应用 schema header MUST 最小侵入，确保顶部仅一条有效 header 且不删除无关内容。

    场景: schema-apply-locates-project-config-from-subdir
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在项目子目录运行 llman self schema apply
      那么 退出码为零
      那么 相对路径 llmanspec/config.yaml 内容包含 yaml-language-server
  @req:r49
  规则: 配置 schema 生成与 YAML LSP 头注释
    对应 spec: config-schemas — 系统 MUST 生成配置 JSON schema 并写入指定路径；MUST 支持 MUST 为全局子集。

    场景: schema-generate-writes-schema-files
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      当 在项目子目录运行 llman self schema generate
      那么 退出码为零
      那么 相对路径 llmanspec/changes/artifacts/schema/configs/en/llman-config.schema.json 存在
  @req:r73
  规则: 首次运行样例生成与 schema 校验
    对应 spec: config-schemas — 首次运行且全局配置不存在时 MUST 生成样例并写头注释，已存在则不改； llman self schema check MUST 校验 schema 与样例实例，无效或样例不匹配则非零； 运行时读取配置 MUST 按 schema 校验，不符合则非零并报告本地化错误。

    场景: first-run-generates-sample-config-with-header
      假如 全局配置目录为空
      当 运行 llman skills
      那么 退出码为零
      那么 全局 config.yaml 存在且含 yaml-language-server

    场景: schema-check-passes-on-fresh-sample
      假如 全局配置目录为空
      当 在临时工作目录运行 llman self schema generate
      那么 退出码为零
      当 运行 llman self schema check
      那么 退出码为零
  @req:r125
  规则: 多技能仓库源配置 schema
    对应 spec: config-schemas — 全局配置 schema MUST 支持 skills.repo[]（每项含 name/path）；MUST NOT 接受旧 skills.dir（schema 校验失败）；skills.repo[] 中路径不存在或不是目录时 MUST 在启动解析阶段输出警告并过滤该条目，不因此失败。

    场景: 多 repo 配置通过 schema 校验
      假如 全局 config.yaml 含 multi-repo skills 配置
      当 在非交互终端运行 llman self schema check
      那么 退出码为零

    场景: 旧 skills.dir 配置 schema 校验失败
      假如 全局 config.yaml 含 legacy-dir skills 配置
      当 在非交互终端运行 llman self schema check
      那么 退出码非零

    场景: 缺失 repo 路径启动时警告并过滤
      假如 全局 config.yaml 含 missing-path skills 配置
      当 在非交互终端运行 llman skills
      那么 stderr 包含 skipping

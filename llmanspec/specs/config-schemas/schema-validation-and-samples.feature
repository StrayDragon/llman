# language: zh-CN
# 对应 spec: config-schemas — 首次运行且全局配置不存在时 MUST 生成样例并写头注释，已存在则不改；
# llman self schema check MUST 校验 schema 与样例实例，无效或样例不匹配则非零；
# 运行时读取配置 MUST 按 schema 校验，不符合则非零并报告本地化错误。
功能: 首次运行样例生成与 schema 校验
  场景: 首次运行生成样例配置
    假如 CLI 启动且全局 config.yaml 不存在
    当 CLI 运行
    那么自动生成样例配置并写入 schema 头注释

  场景: 已存在配置不被覆盖
    假如 CLI 启动且全局 config.yaml 已存在
    当 CLI 运行
    那么该文件保持不变

  场景: schema 校验失败返回非零
    假如 llman self schema check 发现 schema 无效或样例实例不匹配
    当 命令运行
    那么返回非零退出码并报告错误

  场景: 使用真实全局配置作为样例
    假如 全局 config.yaml 存在且用户运行 llman self schema check
    当 校验执行
    那么使用该文件内容作为样例并对照 schema 校验

  场景: 真实配置不可读或不可解析会失败
    假如 全局 config.yaml 存在但无法读取或无法解析为有效 YAML
    当 运行 llman self schema check
    那么返回非零错误
    而且不回退到默认实例作为样例

  场景: 全局配置不符合 schema 时非零
    假如 CLI 读取全局 config.yaml 且内容与 llman-config.schema.json 不一致
    当 CLI 运行
    那么返回非零退出码并报告错误

  场景: llmanspec 配置不符合 schema 时非零
    假如 llmanspec/config.yaml 与 llmanspec-config.schema.json 不一致
    当 命令运行
    那么返回非零退出码并报告错误

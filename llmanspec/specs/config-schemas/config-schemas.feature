# language: zh-CN
# managed by llman sdd partition-migrate
功能: config-schemas

  @req:r1
  场景: 在子目录运行 schema apply 定位到 repo 根
    假如 用户在 repo 的嵌套子目录中运行 llman self schema apply
    当 命令执行
    那么 schema header 被应用到 repo_root/.llman/config.yaml 与 repo_root/llmanspec/config.yaml（当文件存在时）
    而且 不写入子目录下的同名路径

  @req:r1
  场景: 多条 header 行被重写为一条
    假如 某 YAML 文件顶部包含多条 yaml-language-server schema 行
    当 工具执行
    那么 重写为顶部一条正确 header
    而且 保留其余内容不变

  @req:r1
  场景: 生成 schema 文件
    假如 用户运行 llman self schema generate
    当 生成完成
    那么 全局/项目/llmanspec/sdd-eval playbook schema 被写入或刷新到指定路径

  @req:r1
  场景: 头注释缺失时写入
    假如 用户运行 llman self schema apply 且配置文件缺少 schema 头注释
    当 命令执行
    那么 写入对应的 yaml-language-server 头注释

  @req:r1
  场景: 头注释不匹配时修复
    假如 用户运行 llman self schema apply 且 schema URL 与目标不一致
    当 命令执行
    那么 修复为正确的 schema URL

  @req:r1
  场景: 项目配置不含全局专用字段
    假如 llman-project-config.schema.json 被生成
    当 检查 schema
    那么 不包含 skills.dir

  @req:r1
  场景: 首次运行生成样例配置
    假如 CLI 启动且全局 config.yaml 不存在
    当 CLI 运行
    那么 自动生成样例配置并写入 schema 头注释

  @req:r1
  场景: 已存在配置不被覆盖
    假如 CLI 启动且全局 config.yaml 已存在
    当 CLI 运行
    那么 该文件保持不变

  @req:r1
  场景: schema 校验失败返回非零
    假如 llman self schema check 发现 schema 无效或样例实例不匹配
    当 命令运行
    那么 返回非零退出码并报告错误

  @req:r1
  场景: 使用真实全局配置作为样例
    假如 全局 config.yaml 存在且用户运行 llman self schema check
    当 校验执行
    那么 使用该文件内容作为样例并对照 schema 校验

  @req:r1
  场景: 真实配置不可读或不可解析会失败
    假如 全局 config.yaml 存在但无法读取或无法解析为有效 YAML
    当 运行 llman self schema check
    那么 返回非零错误
    而且 不回退到默认实例作为样例

  @req:r1
  场景: 全局配置不符合 schema 时非零
    假如 CLI 读取全局 config.yaml 且内容与 llman-config.schema.json 不一致
    当 CLI 运行
    那么 返回非零退出码并报告错误

  @req:r1
  场景: llmanspec 配置不符合 schema 时非零
    假如 llmanspec/config.yaml 与 llmanspec-config.schema.json 不一致
    当 命令运行
    那么 返回非零退出码并报告错误

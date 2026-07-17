# language: zh-CN
# 对应 spec: config-schemas — llman self schema apply MUST 通过 root discovery 定位 project/
# llmanspec 配置（而非假设 cwd 为根）；应用 schema header MUST 最小侵入，确保顶部仅一条有效
# header 且不删除无关内容。
功能: schema header 经 root discovery 应用且最小侵入
  @req:r1
  场景: 在子目录运行 schema apply 定位到 repo 根
    假如 用户在 repo 的嵌套子目录中运行 llman self schema apply
    当 命令执行
    而且 那么schema header 被应用到 repo_root/.llman/config.yaml 与 repo_root/llmanspec/config.yaml（当文件存在时）
    而且 而且不写入子目录下的同名路径

  @req:r1
  场景: 多条 header 行被重写为一条
    假如 某 YAML 文件顶部包含多条 yaml-language-server schema 行
    当 工具执行
    而且 那么重写为顶部一条正确 header
    而且 而且保留其余内容不变

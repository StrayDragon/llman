# language: zh-CN
# 对应 spec: config-paths r5 — embedding 索引目录位置。
功能: context 索引写入 .context 目录
  @req:r2
  场景: index rebuild 写入 config-dir 下的 .context
    假如 用户运行 llman sdd index rebuild
    当 命令执行完成
    那么 索引文件写入 `<config-dir>/.context/`

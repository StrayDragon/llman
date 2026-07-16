# language: zh-CN
# managed by llman sdd partition-migrate
功能: config-paths

  @req:r5
  场景: index_stored_in_context
    当 user runs `llman sdd index rebuild`
    那么 index files are written to `<config-dir>/.context/`

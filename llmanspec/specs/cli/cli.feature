# language: zh-CN
# managed by llman sdd partition-migrate
功能: cli

  @req:r8
  场景: context_full
    当 agent calls context --task XDG_CONFIG_HOME --paths src/config.rs
    那么 command returns quality=semantic with config-paths in direct

  @req:r8
  场景: context_no_index
    当 agent calls context without index
    那么 command returns quality=unavailable with rebuild hint

  @req:r9
  场景: index_rebuild_check
    当 user runs index rebuild --check
    那么 command outputs Index status without API call

  @req:r10
  场景: context_stale_index
    当 agent calls context with stale index
    那么 command uses keyword retrieval with quality=keyword

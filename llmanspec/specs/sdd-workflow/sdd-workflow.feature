# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-workflow

  @req:r10
  场景: list_json_meta
    当 agent runs list --specs --json
    那么 output includes purpose validScope health staleness

  @req:r47
  场景: context_direct_read
    假如 context returns config-paths in direct
    当 agent receives context output
    那么 agent reads the full config-paths spec

  @req:r48
  场景: triage_behavioral_contract
    当 agent receives task that changes exit code behavior
    那么 agent chooses full SDD workflow with proposal + specs + tasks

  @req:r48
  场景: triage_implementation
    当 agent receives task to fix a typo in README
    那么 agent chooses quick path without change directory

# language: zh-CN
# managed by llman sdd partition-migrate
功能: prompts-management

  @req:r4
  场景: skills_include_quick_and_triage
    当 agent runs llman sdd update-skills --no-interactive --all
    那么 generated skills include llman-sdd-quick and all skills reference context command with async rebuild guidance and triage rules

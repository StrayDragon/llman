# language: zh-CN
# capability: prompts-management
# purpose: 规范 prompts 编排入口、多 app 隔离、冲突策略与 skills 更新引导。
# scope: llmanspec/specs/prompts-management.feature

功能: prompts-management

  @req:r24
  规则: codex target 选择与 claude-code 双 scope 注入
    Codex prompts MUST support `--target project-doc|prompts` (default prompts) with `--override` only for project-doc. Codex and claude-code MUST support global/project dual-scope writes. Claude memory injection MUST use managed-block strategy preserving user content and MUST NOT silently overwrite on read failure.

    场景: codex-prompts-target-flag-surface
      当 运行 llman x codex prompts --help
      那么 退出码为零
      那么 stdout 包含 --target
  @req:r55
  规则: 冲突策略、模板列举、删除确认与 project scope repo root 解析
    Conflict/overwrite policy MUST be consistent (managed targets need interactive confirm or non-interactive `--force`; unmanaged files need secondary confirm). List MUST show readable templates only. Non-interactive rm MUST require `--yes`. Project scope MUST resolve via repo root discovery; missing git root MUST require `--force`.

    场景: claude-code-prompts-help-surface
      当 运行 llman x claude-code prompts --help
      那么 退出码为零
      那么 stdout 包含 --scope
  @req:r77
  规则: prompts 编排入口、app 隔离与 scope 解析
    `llman prompts` MUST run interactively as the orchestrator for `llman x <app> prompts`, keep the `prompt` alias, isolate cursor/codex/claude-code apps, and resolve `--scope` to global|project target sets.

    场景: prompts-orchestrator-interactive-only
      当 运行 llman prompts
      那么 退出码非零
      那么 stderr 包含 interactive-only
  @req:r82
  规则: init --update 含 quick 路径与 triage 引导
    `llman sdd init --update` generated workflow skills MUST include `llman-sdd-quick`, embed `llman sdd context` as the preferred specs lookup with async rebuild guidance when unavailable, and include triage rules distinguishing behavioral-contract vs implementation vs governance vs meta-spec changes.

    场景: workflow-skills-embed-quick-and-context
      假如 已初始化 sdd 项目且 bdd 配置为 "off"
      那么 渲染技能 llman-sdd-quick 存在
      那么 渲染技能 llman-sdd-quick 内容包含 context
      那么 渲染技能 llman-sdd-quick 内容包含 行为合约

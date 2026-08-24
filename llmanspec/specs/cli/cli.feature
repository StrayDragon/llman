# language: zh-CN
# capability: cli
# purpose: 规范 llman CLI 的全局配置目录守卫（仅全局配置命令需要 config-dir）、status 输出，以及 context/index 命令。
# scope: llmanspec/specs/cli

功能: cli

  @req:r13 @human
  场景: 配置守卫范围与命令结构
    - Only subcommands that need global config MUST enforce the dev-project config-dir guard. Authoring commands MUST use unified names (add-req/remove-req/rename-req) with deprecated aliases. Non-core commands MUST live under `sdd project`. Archive MUST require an explicit subcommand. Show MUST support combined output options.

  @req:r42 @human
  场景: status 命令 TOON 输出与 target 解析
    - `llman sdd status` MUST emit pure TOON with kind `llman.sdd.status` including counts/changes/tasks/ops/next. Target resolution MUST follow exact match > prefix match (active first) > multi-match summary > not-found error. Incomplete tasks and pending ops MUST be shown; changes MUST sort by `c<N>-` priority prefix; `--json` MUST remain compatible.

  @req:r8 @human
  场景: Context Command
    - System MUST provide a context subcommand for agent consumption. The command MUST accept --task natural language and/or --paths comma-separated file paths. The command MUST return single JSON with status ok quality qualityNote and spec arrays direct with zScore matchReqs and related. At least one of --task or --paths MUST be required. If embedding index unavailable the command MUST return quality=unavailable with clear error.

  @req:r9 @human
  场景: Index Rebuild Command
    - System MUST provide an index rebuild subcommand. It MUST read all spec files extract per-requirement chunks call embedding API and write index to llmanspec/.context/. It MUST also provide --check flag for freshness check without rebuilding.

  @req:r10 @human
  场景: Context Index Freshness Protocol
    - The context command MUST check index freshness. Read spec_hash from metadata.toml compute current spec hash. If hash matches use semantic retrieval else use keyword with quality=keyword. If index missing return quality=unavailable with rebuild instruction. Context MUST be read-only.

  @req:r112 @human
  场景: Change 名参数前缀匹配
    - llman sdd show/validate/graph/change 等所有接收 change 名的命令，在解析 change id 时 MUST 遵循前缀匹配解析：1) 精确匹配活跃 changes（input 即为完整 id）；2) 前缀匹配活跃 changes（目录名前缀匹配）；3) 前缀匹配归档 changes。精确优先生效。MUST NOT 使用 substring contains 模糊匹配（避免意外命中子串）。前缀（非精确）命中时，命令 SHALL 在输出中提示实际命中的 change（格式 `'<input>' -> '<resolved>' (prefix match)`，人类可读输出走 stderr）；精确命中时不输出该提示。`--json` 输出 SHALL 包含 `matchedViaPrefix` 布尔字段（前缀命中为 true，精确为 false）。

  @req:r112 @executable
  场景: prefix-match-baseline
    假如 存在 active change 和 archived change 且含 c123-fix-bug
    当 用前缀运行 llman sdd show c12
    那么 退出码为零
    那么 对应的完整 change 被找到且输出正确

  @req:r112 @executable
  场景: prefix-match-hint
    假如 存在 active change 和 archived change 且含 c123-fix-bug
    当 用前缀 c123 运行 llman sdd show c123
    那么 stderr 包含 'c123' -> 'c123-fix-bug' (prefix match)
    当 用前缀 c123 运行 llman sdd show c123 --output json
    那么 stdout 含 JSON 键 matchedViaPrefix
    那么 stdout 的 JSON 键 matchedViaPrefix 为 true

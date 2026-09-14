# language: zh-CN
# capability: cli
# purpose: 规范 llman CLI 的全局配置目录守卫（仅全局配置命令需要 config-dir），以及 context/index 命令。
# scope: llmanspec/specs/cli.feature

功能: cli

  @req:r13 @human
  场景: 配置守卫范围与命令结构
    - Only subcommands that need global config MUST enforce the dev-project config-dir guard. Authoring commands MUST use unified names (add-req/remove-req/rename-req) with deprecated aliases. Non-core commands MUST live under `sdd project`. Archive MUST require an explicit subcommand. Show MUST support combined output options.

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

  @req:r56 @human
  场景: 外部子命令自动发现与委托
    - The CLI MUST treat an unknown first token `<name>` as an external subcommand: resolve executable `llman-<name>` on `PATH` (directory order, first hit; file AND any-execute-bit on Unix) and delegate to it as a child process. Built-in subcommands MUST always take precedence. The contract is language-agnostic and MUST forward: (1) argv after `<name>` verbatim without reparsing; (2) environment inherited, plus one normalization — the hub-resolved config dir (priority per config-paths r17) injected as `LLMAN_CONFIG_DIR` for the child; when neither `-C/--config-dir` nor `LLMAN_CONFIG_DIR` is provided, nothing is injected; (3) stdio inherited; (4) cwd inherited; (5) exit code exactly propagated, and `128+signum` when the child dies by signal on Unix. The `LLMAN_*` prefix is reserved across processes. When no `llman-<name>` exists on `PATH`, the CLI MUST print an unrecognized-command error to stderr (exit 1, per errors-exit r22) and SHALL append a hint listing discovered `llman-*` commands on `PATH`. `RequiresGlobalConfig` guard MUST NOT apply to external delegation.

  @req:r56 @executable
  场景: external-delegation-baseline
    假如 PATH 前置目录含可执行假插件 llman-fake-echo
    当 运行 llman fake-echo --flag value
    那么 退出码为零
    那么 stdout 包含 args=--flag value

  @req:r56 @executable
  场景: external-delegation-config-env
    假如 PATH 前置目录含可执行假插件 llman-fake-echo
    当 用 -C 临时配置目录运行 llman fake-echo
    那么 假插件进程 env 中 LLMAN_CONFIG_DIR 为 -C 临时配置目录

  @req:r56 @executable
  场景: external-delegation-exit-code
    假如 PATH 前置目录含以退出码 3 结束的假插件 llman-fake-exit
    当 运行 llman fake-exit
    那么 退出码为 3

  @req:r56 @executable
  场景: external-not-found-hint
    假如 PATH 前置目录仅含 llman-real-plugin 而无 llman-no-such-cmd
    当 运行 llman no-such-cmd
    那么 退出码为 1
    那么 stderr 包含 unrecognized
    那么 stderr 包含 llman-real-plugin

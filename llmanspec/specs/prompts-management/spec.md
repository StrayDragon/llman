---
llman_spec_valid_scope:
  - src/
  - tests/
llman_spec_valid_commands:
  - llman sdd validate prompts-management --type spec --strict --no-interactive
llman_spec_evidence:
  - migrated from openspec
---

```toon
kind: llman.sdd.spec
name: "prompts-management"
purpose: "TBD - created by archiving change add-prompts-injection. Update Purpose after archive."
requirements[13]{req_id,title,statement}:
  r1,`llman prompts` 仅作为交互式编排入口,"`llman prompts` MUST 仅在交互环境中运行，并作为对 `llman x <app> prompts` 的编排入口（不承载具体的模板 CRUD 或注入落盘逻辑）。 命令 MUST 提供 `--no-interactive` 选项，用于在非交互环境下输出迁移提示并正常退出。"
  r2,`prompts` 为主命令并保留 `prompt` 别名,CLI MUST 将 `llman prompts` 作为主命令名，并且 MUST 接受 `llman prompt` 作为等价别名。
  r3,"支持 `cursor` / `codex` / `claude-code` 三类 app","系统 MUST 提供以下命令组用于按 app 维度隔离模板并执行注入： - `llman x cursor prompts` - `llman x codex prompts` - `llman x claude-code prompts` 模板 MUST 仍存储在 `$LLMAN_CONFIG_DIR/prompt/<app>/` 下，并在列出/读取/写入/删除时按 app 维度隔离。"
  r4,Codex 支持选择注入目标类型,"当执行 `llman x codex prompts gen` 时，命令 MUST 支持 `--target project-doc|prompts` 用于选择注入目标类型。 若用户未提供 `--target`，默认 MUST 为 `prompts`。 当 `--override` 被提供但 `--target` 不包含 `project-doc` 时，命令 MUST 返回错误。"
  r5,Codex prompts 同时支持 global/project 两种 scope,"当执行 `llman x codex prompts gen` 时，命令 MUST 支持两类注入目标（由 `--target` 控制），并在 `global|project` 两层分别生效： - `--target prompts`：写入 Codex custom prompts 文件 - global scope：`$CODEX_HOME/prompts/<name>.md`（未设置 `CODEX_HOME` 时为 `~/.codex/prompts/<name>.md`） - project scope：`<repo_root>/.codex/prompts/<name>.md` - `--target project-doc`：注入到 Codex project doc 文件（托管块聚合；输出文件名固定，不随模板名变化） - global scope：`$CODEX_HOME/AGENTS.md`（未设置 `CODEX_HOME` 时为 `~/.codex/AGENTS.md`） - project scope：`<repo_root>/AGENTS.md` 当 `--targe"
  r6,Claude Code prompts 同时支持 global/project 两种 scope,"当执行 `llman x claude-code prompts gen` 时，命令 MUST 支持以下 scope 目标： - global scope：`~/.claude/CLAUDE.md` - project scope：`<repo_root>/CLAUDE.md` 命令 MUST 使用托管块策略，仅更新 llman 管理的区段，并保留文件中非托管内容。"
  r7,"通过 `--scope` 选择注入目标范围","当执行 `llman x <app> prompts gen` 时，命令 MUST 将 `--scope` 解析为“目标集合”，并支持以下输入形式： - 重复参数：`--scope global --scope project` - 逗号列表：`--scope global,project` scope 关键字 MUST 为 `global|project`，并按 app 的支持范围进行校验： - `codex`：支持 `global` 与 `project` - `claude-code`：支持 `global` 与 `project` - `cursor`：仅支持 `project` 命令 MUST 不再接受 `user` 与 `all`。 若用户未提供 `--scope`，默认 MUST 为 `project`。"
  r8,冲突与覆盖策略一致,"当目标文件存在时，命令 MUST 使用一致的冲突与覆盖策略： - 对完全托管的目标（例如 codex custom prompts、cursor rules），命令 MUST 在交互环境中提示确认是否覆盖；在非交互环境 MUST 拒绝覆盖并返回错误，除非用户显式启用 `--force`。 - 对托管块注入目标（例如 codex project-doc、claude-code memory），若目标文件存在且不包含 llman 托管块标记： - 交互模式 MUST 执行二次确认，任一确认未通过都 MUST 放弃该目标写入。 - 非交互模式 MUST 拒绝该目标写入，除非启用 `--force`。"
  r9,模板列举与读取在扩展名维度一致,对每个 `<app>`，`llman x <app> prompts list` MUST 只展示 `llman x <app> prompts gen` 能够以确定方式读取的模板。命令 MUST 忽略该 app 模板目录中不属于该 app 支持扩展名集合的文件（例如备份文件、不同扩展名的同名文件）。
  r10,非交互删除必须显式确认,"`llman x <app> prompts rm` MUST 支持 `--yes` 用于跳过确认提示。若处于非交互环境，命令 MUST 拒绝删除并返回错误，除非用户显式提供 `--yes`。"
  r11,Claude memory 注入在读取失败时不得静默覆盖,"当向 `CLAUDE.md` 注入托管块时，如果目标文件已存在但无法作为 UTF-8 文本读取，命令 MUST 直接失败并 MUST NOT 对该路径进行写入。"
  r12,Project scope 必须通过 repo root 发现解析,"对于选择了 project scope 的目标路径，命令 MUST 通过向上查找父目录定位 git repo root，并 MUST 将 repo 内任意子目录视为有效 project 上下文。 project scope 的写入目标 MUST 为： - codex + `--target prompts`：`<repo_root>/.codex/prompts/<name>.md` - codex + `--target project-doc`：`<repo_root>/AGENTS.md` 或 `<repo_root>/AGENTS.override.md` - claude-code：`<repo_root>/CLAUDE.md` - cursor：`<repo_root>/.cursor/rules/<name>.mdc`"
  r13,Project scope 在无 git root 时必须显式 force,"当用户请求 project scope 但命令无法发现 git repo root 时，实现 MUST 默认拒绝该 project 目标并返回错误，除非用户显式确认/显式提供 `--force` 以允许将当前工作目录视为 project root。 对于同时选择了多个 scope 的执行，命令 MUST 逐目标处理，且 MUST NOT 因 project 目标失败而跳过其他 scope 目标的尝试。"
scenarios[29]{req_id,id,given,when,then}:
  r1,非交互提示迁移,"","用户运行 `llman prompts --no-interactive`","命令输出提示用户使用 `llman x cursor prompts` / `llman x codex prompts` / `llman x claude-code prompts`"
  r2,使用别名调用,"","用户运行 `llman prompt --no-interactive`","行为与 `llman prompts --no-interactive` 等价"
  r3,"app-维度隔离模板","","用户分别对 `llman x cursor prompts upsert` 与 `llman x codex prompts upsert` 执行写入并使用相同 `--name`",两者模板互不覆盖，且 `llman x cursor prompts list` 不显示 `codex` 模板
  r4,"默认-target-为-prompts","","用户运行 `llman x codex prompts gen --template draftpr`","命令按 `--target prompts` 处理"
  r4,"override-仅对-project-doc-生效","","用户运行 `llman x codex prompts gen --target prompts --override --template draftpr`",命令返回错误
  r5,"生成-codex-全局-custom-prompt-文件","","用户运行 `llman x codex prompts gen --target prompts --scope global --template draftpr`",`$CODEX_HOME/prompts/draftpr.md` 被创建或更新
  r5,"生成-codex-项目-custom-prompt-文件","","用户运行 `llman x codex prompts gen --target prompts --scope project --template draftpr`",`<repo_root>/.codex/prompts/draftpr.md` 被创建或更新
  r5,"生成-codex-全局-project-doc-文档","","用户运行 `llman x codex prompts gen --target project-doc --scope global --template common.en`",`$CODEX_HOME/AGENTS.md` 被创建或更新
  r5,"生成-codex-override-project-doc-文档","","用户运行 `llman x codex prompts gen --target project-doc --scope global --override --template common.en`",`$CODEX_HOME/AGENTS.override.md` 被创建或更新
  r6,"生成-claude-code-全局-memory-文档","","用户运行 `llman x claude-code prompts gen --scope global --template project-rules`",`~/.claude/CLAUDE.md` 被创建或更新
  r6,生成并保留用户自定义内容,"","项目 `CLAUDE.md` 已包含用户手写内容，且用户运行 `llman x claude-code prompts gen --scope project --template project-rules`",命令仅更新托管块内容，不删除或改写用户手写内容
  r7,"重复参数选择双-scope","","用户运行 `llman x codex prompts gen --scope global --scope project --template draftpr`",命令同时处理全局与项目目标
  r7,"逗号列表选择双-scope","","用户运行 `llman x claude-code prompts gen --scope global,project --template project-rules`",命令同时处理全局与项目目标
  r7,"cursor-传入不支持-scope","","用户运行 `llman x cursor prompts gen --scope global --template demo-project`",命令返回错误并提示 cursor 仅支持 `project`
  r7,"传入已移除的旧-scope","","用户运行 `llman x codex prompts gen --scope user --template draftpr`",命令返回错误
  r8,"交互模式下-codex-prompts-覆盖确认","",目标 custom prompt 文件已存在且终端可交互,命令提示确认是否覆盖
  r8,交互模式下非托管文件触发二次确认,"","目标文件存在且不包含 llman 托管块，用户在交互模式运行 `llman x codex prompts gen --target project-doc`",命令执行二次确认
  r8,"非交互模式下非托管文件未提供-force","","目标文件存在且不包含 llman 托管块，终端不可交互且命令未提供 `--force`",命令拒绝该目标写入并返回错误
  r9,模板目录包含混合扩展名,"",模板目录同时包含支持的模板文件与不相关文件（例如不同扩展名的备份）,`llman x <app> prompts list` 仅展示可读取模板
  r10,"非交互-rm-未提供-yes","","终端不可交互且用户运行 `llman x cursor prompts rm --name foo`","命令返回非零错误并提示需要传 `--yes`"
  r10,"非交互-rm-提供-yes","","终端不可交互且用户运行 `llman x cursor prompts rm --name foo --yes`",模板被删除且不会出现交互提示
  r11,"既有-claude-md-不可读","","`<repo_root>/CLAUDE.md` 存在但无法作为 UTF-8 读取","`llman x claude-code prompts gen --scope project --template <name>` 返回错误且文件未被修改"
  r12,"在-repo-子目录中运行-codex-project-prompts","","用户在 repo 子目录中运行 `llman x codex prompts gen --target prompts --scope project --template draftpr`",输出被写入 `<repo_root>/.codex/prompts/draftpr.md`
  r12,"在-repo-子目录中运行-codex-project-project-doc","","用户在 repo 子目录中运行 `llman x codex prompts gen --target project-doc --scope project --template common.en`",输出被写入 `<repo_root>/AGENTS.md`
  r12,"在-repo-子目录中运行-cursor-project-scope","","用户在 repo 子目录中运行 `llman x cursor prompts gen --scope project --template demo-project`",输出被写入 `<repo_root>/.cursor/rules/`
  r13,"非交互无-git-root-且仅-project-scope","","终端不可交互、当前目录不在 git repo 内且用户运行 `llman x codex prompts gen --scope project --template draftpr`","命令返回非零错误并提示需要 `--force`，且不会写入 project 目标"
  r13,"非交互无-git-root-且选择-global-project","","终端不可交互、当前目录不在 git repo 内且用户运行 `llman x codex prompts gen --scope global --scope project --template draftpr`",命令会尝试写入 global 目标
  r13,"非交互无-git-root-但提供-force","","终端不可交互、当前目录不在 git repo 内且用户运行 `llman x codex prompts gen --scope project --force --template <name>`",命令将 `cwd` 视为 project root 并写入到 `<cwd>/.codex/prompts/`
  r13,"交互无-git-root-且用户拒绝强制执行","","终端可交互、当前目录不在 git repo 内且用户运行 `llman x codex prompts gen --scope project --template <name>` 并在提示中选择不强制执行",命令安全退出且不会写入任何文件
```

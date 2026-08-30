# Design: add-sdd-cmdref-generation

## 决策

- **D1 SSOT 选择**：命令「存在性」来自 clap 命令树（`SddCommands::command()` 递归
  walk，跳过 `hide=true` 如 Delta stub）；命令「一句话说明」来自 i18n
  `sdd.cmdref.<dotted-path>`（en + zh-Hans），缺 key 回退 clap about。
  理由：存在性手写必漂移（spec-md2toon 实证）；文案放 i18n 可双语且受
  `just qa` 死 key 审计保护，clap doc comment 只做 en 兜底。
- **D2 渲染期注入而非运行时**：`build_template_vars` 增加
  `sdd_command_reference`（按 locale 渲染成 Markdown 块），模板
  `{{ unit("skills/sdd-commands") }}` → `{{ sdd_command_reference }}`。
  产物静态、随版本变化，对话缓存命中友好（用户决策，元 skill 缓行）。
- **D3 零兼容**：静态单元文件直接删除，无过渡并存期；旧语法（spec-md2toon、
  toon 载体叙述）一并清扫。用户以版本切换获得对应 skills。
- **D4 协议尾缀内容学**：六节标题保留（r32 合约），正文压到 3–5 行——
  只写「本 skill 未覆盖的自检项」，与正文复读的条目一律删除；Ethics 一行化
  （risk_level + 禁止项引用正文硬约束），apply-cycle 的具体写法为范本。
- **D5 mermaid 收敛**：r96 修订（已 rules_edit_acked）：per-skill 导航图改
  一行文字；权威生命周期 mermaid 只保留 propose（渲染产物）与根 AGENTS.md。
- **D6 propose 非阻塞 id**：硬约束从「写文件前必须与用户确认 change id」改为
  「用户给出则用之；否则按 r99 推导规则生成合法 id 并宣布后继续」。
  与 draft 路径行为对齐，消除最后一个阻塞式提问。
- **D7 stage-guard 收窄**：apply/verify 保留全表（它们执行门禁判定）；
  explore/apply-cycle 换成一行「用 show --json 的 stage/readyToImplement 判定，
  全表见 apply skill」。

## 测试边界（与用户确认：仅单测）

- clap 遍历器单测：注册表包含全部可见叶子路径、不含 hide 项、about 非空。
- 渲染单测：en 与 zh-Hans 产物均含生成块；one-liner 缺 key 回退 clap about；
  生成块不含已删除子命令（以 spec-md2toon 为回归样本）。
- 模板门禁：`check-sdd-templates` locale parity 照旧（单元增删双 locale 对称）。
- 无新 @executable 场景；BDD harness 面不变。

## 非目标

- 动态元 skill / `agent prompt` CLI（草案 `not-planning-changes/add-meta-skill-dynamic-prompts` 缓行）。
- 任何旧语法兼容层。
- 协议单元的删除（r32 合约面不动，只动内容）。

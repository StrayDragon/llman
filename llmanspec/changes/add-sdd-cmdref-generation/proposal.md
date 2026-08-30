---
depends_on: []
rules_edit_acked: true
---

# SDD skill 命令参考自动生成 + 路线一静态瘦身

## Why

复核（11 skill / 1558 行 / ~99KB ≈ 25k tokens）发现两类问题：

1. **手写命令表结构性漂移**：`sdd-commands` 静态单元在 9 个渲染 skill 里推广
   `llman sdd project migrate --kind spec-md2toon`，而 CLI 只接受
   `toon2features`（r115 移除了其余 kind）。propose 生命周期图还写着
   「编辑 live llmanspec/specs/**<br/>toon / feature」——toon 已不是合约载体。
   根因：命令事实有 clap 这份 SSOT，skill 里却是第二份手写拷贝。
2. **39% 逐字重复尾缀**：命令表 15% + 结构化协议尾缀 15% + Ethics 占位 5% +
   mermaid 4% + stage-guard 5%。协议尾缀与正文互相复读；Ethics 在 9 个 skill
   里是无消费者的占位模板；每个 skill 的导航 mermaid 只为说一句「你在 X」。
3. **propose 阻塞式询问 change id**：与 r99 的 draft 路径行为不一致，是
   「不要问要不要继续」原则下唯一残留的阻塞式提问。

方向（用户决策）：静态 skill、渲染期生成（缓存友好，不做运行时元 skill）；
只保留当前唯一语义，不兼容过去任何语法——用户切换版本即获得对应 skills。

## What Changes

**命令参考生成（核心）**

- CLI 侧新增 clap 命令树遍历器：以 `SddCommands` 的 clap Command 为 SSOT
  产出可见子命令注册表（路径 + about + 关键 flag）。
- `init --update` 渲染期把注册表渲染为 `sdd_command_reference` 变量注入
  MiniJinja：one-liner 取自 i18n（`sdd.cmdref.<dotted-path>`，en 与 zh-Hans
  双语）；缺 key 时回退 clap about（en），渲染不失败；死 key 由既有 i18n
  key 审计（just qa）兜底。
- 模板中 `{{ unit("skills/sdd-commands") }}` 全量替换为
  `{{ sdd_command_reference }}`；静态单元文件删除（零兼容，无过渡期）。
- 顺带审计并优化薄弱的 clap doc comments（en 基线文案）。

**路线一静态瘦身（打包）**

- 协议六节保留（r32），内容重写为 3–5 行精确自检句，删除与正文复读的条目。
- Ethics Governance 从占位模板改为每 skill 一行具体声明（apply-cycle 范本）。
- 每个 skill 的导航 mermaid 换成一行文字（「你在 X → 下一步 Y」）；权威
  生命周期 mermaid 仅保留 propose 与 AGENTS.md 两处（需修订 r96，已 ack）。
- stage-guard 表只留在 apply/verify；explore/apply-cycle 换一行等价指引。
- propose 的「硬约束」改为推导 change id 并宣布（复用 r99 的推导规则），
  MUST NOT 阻塞询问用户；用户已给出 id 时直接采用。

## Capabilities

- `sdd-structured-skill-prompts`：新增 2 条 @human 规则（命令参考生成 SSOT、
  propose 非阻塞 id 推导）；修订 r96（导航 mermaid → 文字行，权威图保留
  propose/AGENTS.md）——锁定规则修订，frontmatter 已带 rules_edit_acked。
- `sdd-template-units-and-jinja`：新增 1 条 @human 规则（生成式变量取代静态
  单元的装配语义与回退规则）。

## Impact

- 受影响范围：`crates/llman-sdd`（clap 遍历器 + 渲染注入 + clap 文案）、
  `templates/sdd/{en,zh-Hans}/`（模板 + 单元增删）、`locales/app.yml`
  （`sdd.cmdref.*` 双语段）、渲染产物全量 resync、2 个 live `.feature`。
- 行为变化：渲染 skill 的命令参考与 CLI 永远同源（新增/删除子命令自动反映）；
  skill 正文体量预期 99KB → ~65KB（-34%）。
- 测试口径（用户确认）：仅 Rust 单测——注册表与 clap 树一致性、双语渲染、
  回退语义；模板 locale parity 门禁照旧；无新 @executable 场景。
- 不做：动态元 skill（用户缓行：init --update 天然更新，避免破坏对话缓存命中）；
  任何旧语法兼容。

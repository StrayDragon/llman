---
depends_on: []
title: 注释工具组（llman tool comments overview / clean-useless）
branch: sdd/add-preview-file-head-comment
base_branch: main
base_sha: d534aff97783f78b319d17365aae4063ee9a9bb1
---

# 注释工具组

> 探索阶段已拍板：目录/glob 尊重 gitignore；显式点名的文件始终处理。路径不存在且 glob 零命中 → 非零退出；目录存在但无可抽头注释 → 空 stdout、exit 0。命令面改为名词组 `comments`（用户确认，含把既有 clean 一步挂入）。

## Why

AGENTS.md 不应维护脚本/文件清单。文件头模块注释（或连续普通注释的第一段）才是描述的 SSOT。Agent 需要一条短命令，按 TARGET（目录/文件/glob）动态抽出头注释纵览。把长 kebab `preview-file-head-comment` 和既有 `clean-useless-comments` 并成麻花一级命令会继续膨胀；`agents-md` 已证明「名词组 + 动作」可扩展。

## What Changes

- CLI 新增组 `llman tool comments`，叶子：
  - `overview <TARGET>...`：文件头第一段注释 TSV 纵览（新能力）
  - `clean-useless`：既有无用注释清理的规范入口（与 `clean-useless-comments` 同一 runner）
- 旧 `llman tool clean-useless-comments` 与别名 `cuc` MUST 仍可调用（clap hidden alias），行为与 `comments clean-useless` 相同；`llman tool --help` 主列表只展示 `comments`。
- 配置键 `tools.clean-useless-comments` 不变。
- `overview` 的 TARGET 判定：存在的文件（始终处理，忽略 gitignore）→ 存在的目录（递归、尊重 gitignore）→ 含 `*?[{` 的 glob（`globset`：`**` 与 `{sh,py}`；命中集尊重 gitignore）。
- 多 TARGET 并集、按相对 cwd 的路径排序去重。
- 文件头嗅探（与扩展名解耦）：跳过首行 `#!`；识别 `#` / `//` / `--` / `/* */` / `"""` `'''`；空行即停（第一段）；剥装饰与 basename 前缀。
- 无头注释、二进制（NUL / 非 UTF-8）静默跳过。stdout 仅 TSV（无横幅）。
- 零命中：路径既不存在、glob 又零命中 → stderr 错误、非零退出；目录存在但无可抽头 → 空 stdout、exit 0。
- v1 `overview` 不接 `.llman/config.yaml`。不做 tree-sitter、不做全文件注释汇总、不写 AGENTS 托管块。
- 本仓库 AGENTS.md 只加一行 `llman tool comments overview 'scripts/**/*.{sh,py,lua,js,mjs,ts}'`。

## Capabilities

- 新建 `tool-comments`：组面、`overview` TARGET 解析与抽取、`clean-useless` 规范入口与旧名 hidden alias。
- `tool-clean-comments`：清理行为合约不变（不改既有规则块）。

## Impact

- 代码：`src/tool/command.rs`、`src/cli.rs` 分发、新模块 `src/tool/comments.rs`、`locales/app.yml`。
- 测试：解析纯函数单元测试 + `tests/it` + BDD（复用 `运行 llman {args:rest}` seam）。
- README 命令表：`just readme`（`tool` 表出现 `comments`，旧一级名不再出现）。
- 无配置 schema 变更，无需 migrations。

## Open Questions

无。

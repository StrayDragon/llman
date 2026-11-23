# Design: llman tool comments

## 决策记录

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 命令面 | `llman tool comments <action>` | 避免长 kebab；对齐 `agents-md` |
| overview 叶子 | `overview` | agent 入口；比 `preview` 少 dry-run 歧义 |
| clean 叶子 | `clean-useless` | 与组名并列，一步挂入既有能力 |
| 旧 CLI | hidden alias：`clean-useless-comments` / `cuc` | 已发布命令不断；help 主列表只留 `comments` |
| 配置键 | 仍用 `tools.clean-useless-comments` | 避免 schema / 用户配置迁移 |
| gitignore | 目录与 glob 尊重；显式文件始终处理 | 已确认 |
| 零命中 | 不存在且 glob 空 → 非零；目录空头 → exit 0 | 已确认 |
| 注释解析 | 内容嗅探，不按扩展名分派 | 多种扩展共用一套头注释状态机 |
| 依赖 | `ignore` + `globset` | `glob` crate 无 `**` / `{a,b}` |
| overview 配置 | v1 无新 tools 段 | 避免 schema 膨胀 |
| 输出 | overview 仅 TSV | 横幅会污染 agent 解析 |

## CLI 形状

```text
llman tool comments overview <TARGET>...
llman tool comments clean-useless [既有 clean-useless-comments 旗标]
llman tool clean-useless-comments ...   # hidden，同一 runner
llman tool cuc ...                      # hidden
```

`CommentsArgs` 嵌套 subcommand；顶层 `CleanUselessComments` 标记 `hide = true`，handler 与 `comments clean-useless` 共用 `clean_comments::run`。

## overview TARGET 解析

```
for each TARGET:
  if path exists as file → include (even if gitignored)
  else if path exists as dir → WalkBuilder(dir), standard_filters (gitignore)
  else if TARGET contains glob chars *?[{ → globset over walk from cwd, gitignore
  else → not found (fatal)
union → sort by relative cwd path → extract_header each
```

- glob 字符检测在「路径不存在」之后。
- 相对路径相对 **cwd**。
- stdout 只写 TSV 行，不写状态横幅。

## 头注释状态机

纯函数 `extract_header(source, basename) -> Option<String>`，语义对齐探索阶段 awk：

1. 首行 `#!` 跳过。
2. 去行首空白；尚未进入模式时：空行 → None；否则按前缀选 mode；否则 None。
3. 行注释 / 块注释 / docstring 只要第一段。
4. 若文本以 basename 开头，剥 basename 与 `[-:=|]`。

二进制：含 NUL 或非法 UTF-8 → 跳过该文件。

## 模块边界

- `src/tool/comments.rs`：overview 解析 + walk + 打印。
- 不复用 `tree_sitter_processor`；不改 `CommentProcessor`。

## 测试 seam

- CLI 子进程：`tests/bdd/steps/shared.ts` 的 `运行 llman {args:rest}`。
- 新 Given：`含头注释文件与 gitignore 的临时项目`（TempDir）。
- `extract_header` 模块内单元测试覆盖五种风格。

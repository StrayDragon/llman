# 抓取清单（用于并行）

更新时间：2026-02-05

目标：只抓取与**配置/文件位置/解析顺序/优先级覆盖/MCP**直接相关的原始内容（原文片段），并记录来源信息，供主线文档结构化整理。

## A. Claude Code（Anthropic）

- 配置入口
  - 配置文件位置、格式、字段（如有）
  - CLI flags 与 env vars 列表
  - 覆盖优先级（flags > env > config 等）
- 上下文/指令文件
  - 项目级：例如 `CLAUDE.md`、`.claude/**`（如官方有约定）
  - 用户级：例如 `~/.claude/**`
  - 解析顺序：就近/递归/合并策略
- MCP / 工具
  - 如何启用 MCP
  - server 定义文件位置与 JSON/TOML 格式
  - 多 server 的加载顺序/冲突策略
- 运行时/网络开关
  - 代理/自定义 base url（如 `ANTHROPIC_BASE_URL`）
  - “非必要流量”相关开关（如 `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`）

建议来源（优先）：Anthropic 官方文档 + Claude Code 仓库/Release notes（如有）。

## B. Codex CLI（OpenAI）

- 配置入口
  - 用户级/全局配置文件位置与格式
  - 仓库级配置（如 `AGENTS.md`）与其作用域规则
  - CLI flags/env vars 的覆盖关系
- 上下文解析顺序
  - `AGENTS.md` 的发现/继承规则（目录树覆盖）
  - prompts / hooks（如存在）加载顺序
  - skills 的发现与启用（目录、命名、targets）
- MCP
  - MCP server 配置入口与格式
  - 工具暴露/权限（sandbox + approvals）相关规则
- x cmds / start a cli
  - 如有 “x 命令/子命令” 或 “启动交互 CLI” 的配置点与顺序
- SDD
  - 如官方存在 Spec-Driven Development（SDD）命令/工作流，抓取其配置与目录约定

建议来源（优先）：OpenAI Codex CLI 官方仓库 README / docs / wiki / discussions。

## C. Cursor

- Rules / Prompts
  - `.mdc` 规则文件格式（frontmatter 字段说明）
  - 规则文件位置（项目/用户/全局）与同步方式
  - 解析顺序：`alwaysApply`、`globs` 匹配、冲突合并策略
- MCP
  - Cursor 的 MCP server 配置入口（UI/文件）
  - JSON 格式与字段（command/args/env 等）
  - 多 server 的加载/优先级与冲突策略
- 其他配置
  - workspace settings、规则共享、团队策略（如有）

建议来源（优先）：Cursor 官方文档（rules + MCP）。

## 产物格式（放入 `_raw/<tool>/`）

每个来源一份文件，文件名建议：

`<domain>__<path_or_topic>__2026-02-05.md`

文件头部（必须）：

```
source_url: ...
fetched_at: 2026-02-05T...
title: ...
version_or_last_updated: ...
```

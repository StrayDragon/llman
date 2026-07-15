# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## 项目上下文

<!-- 在此添加项目上下文：技术栈、约定、约束等 -->

## Artifact 规则

<!-- 在此添加按 artifact 的规则，例如：
- proposal: 提案保持在 500 字以内，必须包含"非目标"章节
- tasks: 每个任务不超过 2 小时
-->

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd init --update` 刷新。

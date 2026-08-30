---
name: "llman-sdd-onboard"
description: "了解 llman SDD 工作流并完成项目入门。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD 入门

使用此 skill 让你快速了解 llman SDD 工作流。

## 步骤
1. 阅读 `llmanspec/config.yaml` 了解项目上下文、约定与规则。
2. 使用 `llman sdd list --specs --json` 了解项目中的 specs 概览。
   - 或者使用 `llman sdd context --task "<任务描述>" --paths "<路径>"` 获取与当前任务相关的 specs。
   - 如果 context 返回 `quality: "unavailable"`，先运行 `llman sdd index rebuild` 重建索引（默认 backend 为 `pageindex`；检索需 `LLMAN_SDD_INDEX_CHAT_MODEL`，但重建不需要）。
3. 根据 context 的 `direct`/`related` 分类，只读 target spec 全文。
4. 判断变更规模（见 triage 规则），决定走完整 SDD 流程或快速路径。
5. 按路径推进：
   - **完整路径**：Designed 规划壳 → Branch binding → Specs landing（或 `skip_specs_landing`）→ `readyToImplement=true` → apply → verify → archive（Skill 导航：propose → apply → verify → archive）。
   - **快速路径**：不改 MUST/SHALL，直接改代码 commit（触及 live specs 须已绑定分支，见 `llman-sdd-quick`）。
6. 使用 `llman sdd graph` 可视化变更依赖关系（depends_on/blocks）。

> 命令细节用 `llman sdd <cmd> --help` 查看；命令参考以 CLI 为准，skill 不内嵌命令表（r139）。

## 备注
- `llmanspec/config.yaml` 包含项目上下文、规则、locale 与 skills 路径。
- locale 仅影响模板与 skills，CLI 仍为英文。
- 使用 `llman sdd init --update` 刷新技能。

{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}

---
id: update-skill-bdd-mode-conditioning
depends_on: []
branch: feat/update-skill-bdd-mode-conditioning
base_sha: 7aaab05c9eabcfd6b26b4b2e7b0082d959997154
checkpointed: true
checkpoint_sha: 7aaab05c9eabcfd6b26b4b2e7b0082d959997154
---

# Proposal: Skill 按 BDD 模式条件渲染 + 元信息一致性门禁 + context 懒刷新

## Why

当前 SDD skill 模板是 **BDD-on / BDD-off 混写一份正文**：

1. Agent 在 BDD-on 项目仍被 description 引导去写「delta specs」；propose 引用未安装的
   `llman-sdd-continue`；verify 仍优先读 `changes/<id>/specs/`。
2. 安装产物无 `bdd_mode` 元信息，`config.yaml` 切换 `bdd:` 后已安装 skills 会与配置漂移，
   且 `validate` / `init --update` 无法发现。
3. `llman sdd context` 在 index stale 时直接 `quality: unavailable`，强迫 agent 手动 rebuild，
   与「懒加载下次调取时自动更新」预期不符。
4. Skill 尾部命令表 / 空 Ethics 占位浪费 token；mermaid 管线图需保留（量化表明省略后易幻觉）。

应急方案（本次）：**同一套模板源 + MiniJinja 按 `bdd_enabled` 条件渲染**，产物写入
`metadata.llman_sdd.{bdd_mode,skill_set}`，并在所有相关触发点做一致性 ERROR。
长期方向（另见 draft `add-meta-skill-dynamic-prompts`）：元 skill + `llman sdd` 动态提示词。

## What Changes

### 1. Skill 模板条件化（B2）

- 保持单一 `templates/sdd/{locale}/skills/*.md` 源文件。
- 用 MiniJinja `{% if bdd_enabled %}` / `{% else %}` 拆开 BDD-on / BDD-off 专属段落
  （propose/apply/verify/archive/explore/quick 等主路径；保留 mermaid）。
- `update-skills` / `init --update` 按当前 `config.yaml` 是否含 `bdd:` 渲染对应正文。
- description / 交叉引用：
  - BDD-on：不以「delta specs」为主要产物表述；live specs + attach/finalize。
  - BDD-off：delta + archive merge；不把 attach/finalize 写成必经。
  - 对 optional skills（continue/ff/sync/…）：仅在 `extra_skills` 启用时推荐为下一步，
    否则给出「补齐 artifact / 启用 extra_skills」替代指引。

### 2. YAML 元信息（C1）

每个托管 `llman-sdd-*` SKILL.md frontmatter MUST 含：

```yaml
metadata:
  version: "<cli version>"
  llman_sdd:
    bdd_mode: on   # on | off — 必须与 config 是否含 bdd: 一致
    skill_set: default  # default | optional
```

### 3. 一致性门禁（D1 — 全触发）

下列入口 MUST 检查已安装托管 skills 的 `llman_sdd.bdd_mode` 与项目配置一致
（缺字段 / 值非法 / 与 `bdd:` 有无不一致 → **ERROR**，stderr 含可操作修复：
`llman sdd init --update` 或 `llman sdd update-skills`）：

- `llman sdd validate`（单 change / 单 spec / `--all` / `--specs`）
- `llman sdd init --update`
- `llman sdd update-skills`（写入前或写入后校验；写入后产物必须自洽）

### 4. Context 懒刷新（E3）

`llman sdd context` 在 pageindex index **stale 或 missing** 时 MUST 自动执行一次
tree rebuild（无需 chat model），再继续 retrieval。MUST NOT 仅因 stale/missing 返回
`quality: unavailable`。corrupted 时 SHOULD 尝试 rebuild；chat model 未配置时仍可在
rebuild 成功后以既有 `api_error` 失败（与今日一致）。

### 5. 文档

新增 `docs/sdd/`：

- `pipeline-bdd-on.md` — BDD-on agent skill 选型 + Git-native 闭环流程图
- `pipeline-bdd-off.md` — BDD-off 选型 + delta/archive 流程图
- `README.md` — 索引与「应急条件模板 vs 元 skill 方向」说明

### 6. 明确非目标（本次）

- 不实现元 skill / 运行时动态提示词注入（见 draft change）。
- 不默认启用全部 optional skills。
- 不删除 mermaid pipeline 图。

## Capabilities

- `sdd-workflow` — validate / update-skills 一致性门禁与 skill 元信息合约（r95）
- `sdd-structured-skill-prompts` — 条件化正文与 description 准确性（r96）
- `sdd-context` — context 懒刷新（r97）
- `skills-management` — 版本元数据扩展说明（与 r84 协同；可执行场景挂 workflow/skills feature）

## Impact

- **Breaking（可修复）**：旧版已安装 skills 缺 `llman_sdd.bdd_mode` 时，validate / init --update
  将失败，直到用户执行 `init --update`。
- 模板与 `just check-sdd-templates` 需覆盖条件渲染后的双模式抽检。
- BDD 兼容测试（`sdd-bdd-mode-compat` / `sdd_bdd_compat_tests`）需同步门禁行为。
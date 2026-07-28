# Design: add-skill-gate-eval-baseline

## 核心思路：组合而非重造

两套现有 suite 各有半个能力。P1 = 把它们拼起来：

```
sdd_skill_gate_v1 = sdd_apply_v1 的「skill 渲染进 prompt」
                  + sdd_llmanspec_styles_v1 的「sandbox agent + 硬门禁」
                  + 新增「跨版本 A/B 基线对比」
```

## 复用映射

| 需要的能力 | 来源 | 具体函数/文件 |
|---|---|---|
| SKILL.md 渲染进 prompt | `run-sdd-prompts-eval.sh` | `render_skill_prompt()`（:149-187）、`strip_frontmatter()`（:140-147） |
| sandbox 建站（init + git baseline + 隔离 config） | `run-sdd-claude-style-eval.sh` | `init_workspace()`（~:290-307）、`ensure_promptfoo_anthropic_api_key()`（:257-282） |
| 硬门禁断言 | `sdd_llmanspec_styles_v1/assertions/sdd_gate.py` | `get_assert()` → `_hard_validate()`（shell out `llman sdd validate`） |
| 批次聚合统计 | `run-sdd-claude-style-eval.sh` | `summarize_results()`（:473-583）、`aggregate_batch_results()`（:585-790） |
| agentic provider 配置 | `sdd_llmanspec_styles_v1/promptfooconfig.yaml` | `anthropic:claude-agent-sdk` + `working_dir` + `append_system_prompt` |

## 关键设计决策

### D1: 单 workspace（非多 style）

`sdd_llmanspec_styles_v1` 有 3 个 provider（ison/toon/yaml），因为要对比 spec 格式。`sdd_skill_gate_v1` 对比的是 **skill 模板版本**（baseline vs candidate），不是格式——所以用 **2 个 provider**（baseline-skill / candidate-skill），各指向独立 workspace（隔离 SKILL.md 渲染产物）。

### D2: SKILL.md 注入方式

复用 `render_skill_prompt`：
1. 在各 workspace 跑 `llman sdd update-skills --tool codex --skills-only`
2. 取 `.codex/skills/<SKILL_ID>/SKILL.md`，strip frontmatter
3. 包进 `prompts/agent_task.md` 的 system 段（对齐 `run-sdd-prompts-eval.sh:227-241` 的 chat prompt 结构，但改为 agentic）

baseline workspace 用「上一版模板快照」（`--baseline-skill <path-to-snapshot-md>`，runner 临时覆盖 `.codex/skills/<id>/SKILL.md`），candidate 用当前工作区模板。

### D3: MVP 评分 = 硬门禁 + 成本，不含 rubric

- **硬门禁**（确定性，pass/fail）：复用 `sdd_gate.py` 的 `_hard_validate` 逻辑（`llman sdd validate --all --strict`）。
- **成本**（来自 promptfoo 原生 metrics）：token（prompt/completion/total）、turns、cost。
- **不含 LLM-rubric**（P3 再加）：MVP 先靠确定性信号，避免 judge 噪声。

### D4: 基线锚点 = 对比上一版模板（非 golden）

`--baseline-skill` 接受一个 `.md` 文件路径（上一版模板快照，可由 `git show HEAD:templates/.../SKILL.md > snapshot.md` 产生）。runner 在 baseline workspace 用该快照覆盖渲染产物。**不做 golden 参考**（维护成本高，P3）。

### D5: P1 只测 apply skill

apply 是唯一已有部分 eval 覆盖的 skill，且 agentic 任务可设计为「拿到带 tasks.md 的 change shell → 推进实现 → validate 通过」。draft/propose 留 P2（且 P2 的 draft 测例依赖线1 promote-draft-skill 落地）。

## P1 apply 任务设计

agent 拿到：
- 渲染后的 `llman-sdd-apply` SKILL.md（作为 system prompt 主体）
- 一个预置 sandbox：已 init 的 llmanspec 项目 + 一个 `change add-sample/`（含 proposal.md + tasks.md，tasks 有 2-3 个待办，涉及简单代码改动如加一个 CLI 子命令或修一个函数）

agent 应：
1. 读 tasks.md，汇报进度
2. 按任务推进实现（改代码 + 跑测试）
3. 运行 `llman sdd validate --all --strict --no-interactive` 通过

硬门禁断言：workspace 里 `llman sdd validate` 退出码 0 + tasks.md 全勾选（或 git diff 显示预期文件改动）。

## 不做的事（P1 边界）

- 不做持久化基线存储（P3）
- 不做 LLM-rubric 软分（P3）
- 不做 golden 参考锚点（P3）
- 不覆盖 draft/propose/quick/verify/archive（P2）
- 不改现有两套 suite（向后兼容）
- 不接 CI（P3，先本地可跑）

## 测试边界（seam）

- **脚本可执行**：`bash scripts/sdd-skill-gate-eval.sh --no-run` 能生成 sandbox + 渲染 prompt（不调 API）。
- **fixture 结构完整**：`agentdev/promptfoo/sdd_skill_gate_v1/` 含 promptfooconfig.yaml（占位符 `__WORKDIR_BASELINE__` / `__WORKDIR_CANDIDATE__` 等）+ tests.yaml + prompts/agent_task.md + assertions/sdd_gate.py（复用或精简版）。
- **wrapper 对齐**：`scripts/sdd-skill-gate-eval.sh` 与现有 `scripts/sdd-prompts-eval.sh` / `scripts/sdd-claude-style-eval.sh` 同模式（exec 到 `agentdev/promptfoo/run-...`）。

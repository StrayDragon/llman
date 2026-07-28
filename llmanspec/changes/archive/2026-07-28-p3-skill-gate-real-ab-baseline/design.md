# Design: p3-skill-gate-real-ab-baseline

## 核心：per-provider prompt override（CRITICAL-1 修复）

promptfoo 原生支持 provider 级 `prompt` override（一个 config 里每个 provider 用不同 prompt）。这正好解决「baseline/candidate 看到不同 skill 文本」的需求——无需两次独立 eval。

### 结构变化

P1/P2（现状，单共享 prompt）：
```yaml
providers:
  - id: ...baseline
  - id: ...candidate
prompts:
  - file://prompts/agent_task.md   # 两个 provider 共享，都含 candidate skill
```

P3（per-provider override）：
```yaml
providers:
  - id: ...baseline
    prompt: file://prompts/agent_task_baseline.md   # 含 baseline skill 快照
  - id: ...candidate
    prompt: file://prompts/agent_task_candidate.md  # 含 candidate skill（当前工作区）
prompts:
  - file://prompts/agent_task.md   # 保留为 fallback（未被 override 时用）
```

### runner 改动

`compose_agent_task` 调两次：
1. baseline skill prompt → `prompts/agent_task_baseline.md`
2. candidate skill prompt → `prompts/agent_task_candidate.md`

两个文件都用相同的 task_prompt（只有 skill 文本不同）。这保证 A/B 只测 skill 模板差异，不测任务差异。

### 无 --baseline-skill 时的退化

不指定快照时，baseline == candidate（两文件内容相同），A/B 退化为重复测量——但这是用户显式选择「不对比」的合理行为，不是 bug。

## git 快照持久化基线

### 目录结构

```
agentdev/promptfoo/baselines/
  README.md          # 说明如何产生快照
  <skill>-<ref>.md   # 例如 llman-sdd-apply-HEAD~1.md
```

### 快照产生方式

手动（或 CI）用 git 产生：
```bash
git show HEAD~1:templates/sdd/zh-Hans/skills/llman-sdd-apply.md \
  > agentdev/promptfoo/baselines/llman-sdd-apply-HEAD~1.md
```

runner 的 `--baseline-skill` 直接指向该文件。不做自动化快照存储（避免 eval 副作用污染 baselines/），保持 baselines/ 纯人工/CI 管理。

### 为什么不用 eval 结果历史库

git 快照更简单、与 git 工作流一致、可 review。eval 结果历史库（趋势对比）是更重的方案，留待有明确需求时再做。

## 测试边界（seam）

- **dry-run**：`--baseline-skill <snap> --no-run` 生成两个不同的 prompt 文件（baseline 含快照 skill、candidate 含当前 skill）。
- **promptfooconfig**：两 provider 各有 `prompt` override，指向对应文件。
- **baselines/**：目录存在 + README 说明清晰。

# Tasks: add-skill-gate-eval-baseline

## 垂直切片 1：fixture 骨架（promptfooconfig + 占位符）

- [x] 1.1 新建 `agentdev/promptfoo/sdd_skill_gate_v1/promptfooconfig.yaml`：2 个 `anthropic:claude-agent-sdk` provider（baseline / candidate），各带 `working_dir` / `LLMAN_CONFIG_DIR` / `PATH` 占位符（`__WORKDIR_BASELINE__` 等），`append_system_prompt` 对齐 styles_v1（含「finish by running validate」）
  - 验证：YAML 合法；占位符命名与 runner 约定一致
- [x] 1.2 新建 `agentdev/promptfoo/sdd_skill_gate_v1/tests.yaml`：1 个 apply agentic 测试用例（`task_prompt` 描述任务 + `rubric` 占位（MVP 可留简短或空，因硬门禁是主信号））
  (blocked-by: 1.1)
  - 验证：YAML 合法
- [x] 1.3 新建 `agentdev/promptfoo/sdd_skill_gate_v1/prompts/agent_task.md`：模板 `{{ task_prompt }}`，由 runner 在渲染时前置 SKILL.md（system 段）
  - 验证：含 `{{ task_prompt }}` 占位（runner 覆盖此文件注入 SKILL.md）

## 垂直切片 2：硬门禁断言（复用 + 精简）

- [x] 2.1 新建 `agentdev/promptfoo/sdd_skill_gate_v1/assertions/sdd_gate.py`：精简版——复用 `_hard_validate`（shell out `llman sdd validate --all --strict`）+ workspace 推断（从 providerResponse metadata 嗅探，或读 runner 导出的 `SDD_WORKDIR_BASELINE`/`SDD_WORKDIR_CANDIDATE` env）。去掉 style fence 检查（skill-gate 不关心 spec 格式）
  (blocked-by: 1.1)
  - 验证：`python3 -c importlib` 可导入，get_assert 可调用
- [x] 2.2 在 promptfooconfig.yaml 的 `defaultTest.assert` 引用 `file://assertions/sdd_gate.py:get_assert`
  (blocked-by: 2.1, 1.1)
  - 验证：config 引用路径正确

## 垂直切片 3：runner 脚本（组合 sandbox + skill 渲染）

- [x] 3.1 新建 `agentdev/promptfoo/run-sdd-skill-gate-eval.sh`：参数解析（`--skill <id>` 默认 llman-sdd-apply、`--baseline-skill <path>`、`--candidate-skill <path>`（默认当前工作区）、`--model`、`--max-turns`、`--runs`、`--no-run`、`--ui`）
  - 验证：`--help` 输出完整
- [x] 3.2 实现 sandbox 建站：建 2 个临时 workspace（baseline / candidate），各跑 `llman sdd init` + git baseline + 复制 llman binary 到 `.llman-bin/` + 隔离 `LLMAN_CONFIG_DIR` + 预置 add-sample change shell
  (blocked-by: 3.1)
  - 验证：`--no-run` 下两个 workspace 目录存在且含 `llmanspec/` + `changes/add-sample/`
- [x] 3.3 实现 SKILL.md 渲染注入：在各 workspace 跑 `sdd init --update`（触发 update_skills）渲染 `.agents/skills/<skill>/SKILL.md`；baseline 用 `--baseline-skill` 快照覆盖；strip frontmatter 后包进 `prompts/agent_task.md`
  (blocked-by: 3.2)
  - 验证：`--no-run` 下生成的 prompt 含 SKILL.md 正文、不含 frontmatter、units 已展开
  - 注：发现并修正了原计划中的 `sdd update-skills` 误用——该子命令不存在，正确入口是 `sdd init --update`（触发 update_skills::run_with_root）
- [x] 3.4 实现 promptfoo patch + 调用：patch promptfooconfig.yaml 占位符（`__WORKDIR_BASELINE__` 等）→ `promptfoo validate` → `promptfoo eval`（支持 `--runs N` 批次）
  (blocked-by: 3.3, 2.2)
  - 验证：`--no-run` 生成完整 promptfooconfig.yaml（无残留占位符）；promptfoo 缺失时 `--no-run` 优雅降级
- [x] 3.5 实现批次聚合（`--runs >= 2` 时）：复用 styles runner 的 `summarize_results` + `aggregate_batch_results` 模式，写 `summary.{json,md}` + `aggregate.{json,md}`，按 baseline/candidate 维度对比 pass_rate + token/turns/cost 的 mean/median/p90
  (blocked-by: 3.4)
  - 验证：聚合函数代码就绪（真实跑需 API key，留作手动验证）

## 垂直切片 4：兼容入口 wrapper + 文档

- [x] 4.1 新建 `scripts/sdd-skill-gate-eval.sh`：对齐 `scripts/sdd-prompts-eval.sh` 模式，`exec bash "$REPO_ROOT/agentdev/promptfoo/run-sdd-skill-gate-eval.sh" "$@"`
  (blocked-by: 3.1)
  - 验证：`bash scripts/sdd-skill-gate-eval.sh --help` 工作
- [x] 4.2 更新 `agentdev/promptfoo/README.md`：新增 `run-sdd-skill-gate-eval.sh` 条目（入口、用途、与另两套的区别）
  (blocked-by: 4.1)
  - 验证：README 三套 suite 都有描述
- [x] 4.3 新建 `agentdev/promptfoo/sdd_skill_gate_v1/README.md`：说明 P1 范围、占位符、MVP 评分维度、与 P2/P3 的边界
  (blocked-by: 1.1)
  - 验证：README 自洽

## 垂直切片 5：门禁验证

- [x] 5.1 本地 dry-run：`bash scripts/sdd-skill-gate-eval.sh --no-run` 全流程生成成功（sandbox + prompt + patched config）
  (blocked-by: 3.4, 4.1)
  - 验证：无报错，产物结构完整（2 workspace + change shell + SKILL.md 注入 prompt + 无残留占位符）
- [x] 5.2 `just check` 全绿（不引入 Rust 改动，主要是脚本/fixture，确保不破坏现有门禁）
  (blocked-by: 5.1)
  - 验证：502 tests passed
- [x] 5.3 （可选，需 API key + promptfoo）真实跑一次 `--runs 2` 确认 baseline/candidate 对比报告可产出
  (blocked-by: 5.1)
  - 验证：`aggregate.md` 含两维度对比
  - 状态：deferred——环境缺 promptfoo + API key，留作后续手动验证；聚合逻辑（aggregate_batch_results）已就绪并通过 dry-run 结构验证，不阻断本 change

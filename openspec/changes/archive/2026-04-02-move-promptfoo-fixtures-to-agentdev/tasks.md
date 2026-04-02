## 1. 目录迁移

- [x] 1.1 新增 `agentdev/` 顶层目录与说明文件（明确其用途与边界）
- [x] 1.2 将 `artifacts/testing_config_home/promptfoo/` 迁移到 `agentdev/promptfoo/`（包括 `default_models.txt` 与 `sdd_apply_v1/`）
- [x] 1.3 将 Promptfoo 评测入口脚本集中到 `agentdev/promptfoo/`（例如新增 `agentdev/promptfoo/run-sdd-prompts-eval.sh`）
- [x] 1.4 更新 `scripts/sdd-prompts-eval.sh`：作为薄封装入口转发到 `agentdev/promptfoo/`，并使用 `agentdev/promptfoo/` 下的默认模型列表与 fixtures

## 2. 文档与规范

- [x] 2.1 更新与 Promptfoo 路径相关的说明（若存在）以指向 `agentdev/promptfoo/`
- [x] 2.2 运行 `openspec` 校验/状态检查，确保本变更 artifacts 完整

## 3. 回归验证

- [x] 3.1 运行一次 `bash scripts/sdd-prompts-eval.sh --no-gen`（或等价干跑）验证脚本能生成临时目录并引用新路径
- [x] 3.2 确认仓库内不再引用 `artifacts/testing_config_home/promptfoo`（避免残留路径）

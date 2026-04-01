## 1. Promptfoo fixture（Claude Code agentic + multi-style）

- [ ] 1.1 在 `agentdev/promptfoo/` 下新增 fixture 目录（例如 `sdd_llmanspec_styles_v1/`），包含 `promptfooconfig.yaml`、`tests.yaml`、`prompts/`
- [ ] 1.2 Provider 采用 `anthropic:claude-agent-sdk` 并启用 bypass permissions（允许写文件/执行命令），支持更大 `max_turns`
- [ ] 1.3 增加 Python assertions：以 `llman sdd validate --all --strict --no-interactive` 为硬门禁，并补充 fence/style/产物存在性检查
- [ ] 1.4 增加可选 judge 层：human / codex rubric / claude rubric（默认关闭）

## 2. Runner 脚本与临时 workspace（git 快照）

- [ ] 2.1 将 runner 脚本集中到 `agentdev/promptfoo/`（例如新增 `agentdev/promptfoo/run-sdd-claude-style-eval.sh`）
- [ ] 2.2 在 `scripts/` 下保留薄封装入口（例如 `scripts/sdd-claude-style-eval.sh`）转发到 `agentdev/promptfoo/`
- [ ] 2.3 runner 为 ison/toon/yaml 三种风格分别创建独立 workspace，并初始化为 git repo（baseline commit）
- [ ] 2.4 runner 生成 `meta/`：收集每个 workspace 的 `git log/diff/status` 与关键命令输出；汇总 token/cost/turns（若 provider 返回）
- [ ] 2.5 runner 支持 `--repeat/--max-turns/--judge` 等参数透传到 promptfoo 与 fixture 配置

## 3. Docker 环境（可复现 + mirror build args）

- [ ] 3.1 新增 Dockerfile（例如 `agentdev/docker/sdd-claude-style-eval/Dockerfile`），安装 promptfoo + Claude Agent SDK 依赖，并包含运行脚本入口
- [ ] 3.2 Dockerfile 支持 build args 切换阿里云 mirror（apt/npm/pypi），并提供默认值/文档
- [ ] 3.3 新增 docker runner（脚本或 make/just 入口）：支持挂载输出目录与注入 `ANTHROPIC_API_KEY`（以及可选 `OPENAI_API_KEY`）

## 4. 文档与回归验证

- [ ] 4.1 为 `agentdev/` 与该评测脚手架写 README：最小可运行命令、环境变量、输出结构与安全注意事项
- [ ] 4.2 本地 smoke：在至少一个模型上跑 `promptfoo validate` 与一次 eval（或 `--no-run` dry path）验证产物生成

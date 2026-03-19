## 1. CLI 接口与参数解析

- [x] 1.1 在 `llman x codex` 下新增 `agents` 子命令组（import/sync/inject/status）并完成 clap wiring
- [x] 1.2 为 `agents` 命令组补齐公共参数：`--managed-dir` / `--codex-home` / `--agents-dir` / `--only`（repeatable）/ `--mode`（link|copy）/ `--dry-run` / `--yes` / `--force`
- [x] 1.3 实现 `llman x codex agents`（无子命令）进入交互向导（inquire）

## 2. 路径解析与文件选择

- [x] 2.1 实现托管目录解析：默认 `$LLMAN_CONFIG_DIR/codex/agents/`，支持 `--managed-dir` 覆盖
- [x] 2.2 实现目标目录解析优先级：`--agents-dir` > `--codex-home/agents` > `$CODEX_HOME/agents` > `~/.codex/agents`
- [x] 2.3 实现 `--only <name>` 过滤（对 `*.toml` 生效，映射到 `<name>.toml`）

## 3. 安全开关：dry-run 与确认机制

- [x] 3.1 实现 `--dry-run`：输出计划但不落盘（import/sync/inject）
- [x] 3.2 实现非交互写操作需要 `--yes/--force`（否则报错提示可用 `--dry-run`）
- [x] 3.3 实现交互环境写操作确认：未提供 `--yes/--force` 时询问确认；取消则不落盘并退出 0

## 4. 安全覆盖：备份策略

- [x] 4.1 实现备份命名：`<file>.llman.bak.<YYYYMMDDHHMMSS>`（同目录）
- [x] 4.2 在 import/sync 覆盖路径中统一调用备份逻辑，并确保错误信息可定位（包含路径）

## 5. status：只读检查与差异展示

- [x] 5.1 实现 `agents status`：展示托管/目标路径、托管列表、目标侧链接/冲突状态、可注入性提示
- [x] 5.2 确保 `status` 永不落盘（不生成备份文件）

## 6. import：纳入集中托管

- [x] 6.1 实现 `agents import`：读取目标目录一层 `*.toml` 并复制到托管目录
- [x] 6.2 处理托管侧冲突：默认备份后覆盖；不删除未涉及文件

## 7. sync：发布到 Codex agents 目录

- [x] 7.1 实现 `agents sync` 默认 link 模式：逐文件创建/更新 symlink 到托管文件（Unix）
- [x] 7.2 处理目标侧冲突：非期望 symlink 或普通文件 → 备份后替换
- [x] 7.3 实现 `--mode copy`：复制覆盖到目标目录（作为无 symlink 环境兜底）

## 8. inject：模板片段注入到 `developer_instructions`

- [x] 8.1 实现模板读取与拼接：从 llman codex prompts 模板读取，生成带 `## llman prompts: <name>` 的组合内容
- [x] 8.2 实现 TOML 文本内 `developer_instructions = \"\"\"...\"\"\"` 的 marker 注入/替换逻辑（幂等）
- [x] 8.3 缺失 `developer_instructions` 的文件跳过并提示（不导致整体失败）

## 9. 交互向导（inquire）

- [x] 9.1 实现交互向导：选择操作（status/import/inject/sync）并按操作收集 `only/templates/mode` 等参数
- [x] 9.2 在交互向导中执行写操作前展示计划并确认（等价 dry-run 展示 + 确认执行）

## 10. 测试与验收

- [x] 10.1 为 inject 的 marker 更新/插入路径添加单元测试（覆盖：已有 marker、无 marker、缺失字段）
- [x] 10.2 为 `--dry-run` 添加测试：计划输出且不落盘（至少覆盖 sync/import 之一）
- [x] 10.3 为非交互未确认失败路径添加测试（缺 `--yes/--force` 时应报错）
- [x] 10.4 为 import/sync 添加测试：使用 `TempDir` + `CODEX_HOME` 覆盖目标目录；Unix 下覆盖 symlink 行为
- [x] 10.5 手动 smoke：在测试配置目录下跑 `llman x codex agents (wizard)/status/import/sync/inject --help`，确认帮助与错误提示可用

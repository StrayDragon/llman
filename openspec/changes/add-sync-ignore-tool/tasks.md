## 1. 核心数据结构和类型

- [ ] 1.1 定义 `IgnoreFormat` 枚举 (Cursor, ClaudeCode)
- [ ] 1.2 定义 `SyncIgnoreArgs` 结构体包含所有 CLI 参数
- [ ] 1.3 定义 `ClaudeCodeSettings` 和 `Permissions` 结构体用于 JSON 序列化
- [ ] 1.4 定义模式转换错误类型

## 2. 模式转换逻辑

- [ ] 2.1 实现 `cursor_to_claude_code()` 函数
  - 处理基本通配符 (`*.log` → `Read(./*.log)`)
  - 处理目录通配符 (`secrets/**` → `Read(./secrets/**)`)
  - 处理否定模式 (跳过并警告)
  - 处理注释行 (# 开头)
  - 处理空行

- [ ] 2.2 实现 `claude_code_to_cursor()` 函数
  - 解析 `Read()` 规则
  - 提取路径模式
  - 跳过非 Read 规则并警告
  - 移除 `./` 前缀和 `**` 后缀

- [ ] 2.3 实现 `detect_format()` 函数
  - 根据文件名检测格式
  - 验证 JSON 内容是否包含 permissions

- [ ] 2.4 实现 `find_default_source()` 函数
  - 按优先级查找 `.cursorignore`
  - 然后查找 `.claude/settings.json`

## 3. 文件读写操作

- [ ] 3.1 实现 `read_cursorignore()` 函数
  - 读取文件内容
  - 按行分割
  - 过滤注释和空行
  - 返回有效模式列表

- [ ] 3.2 实现 `write_cursorignore()` 函数
  - 写入模式到文件
  - 使用原子写入确保安全
  - 设置适当权限

- [ ] 3.3 实现 `read_claude_code_permissions()` 函数
  - 读取 settings.json
  - 解析 JSON
  - 提取 permissions.deny 数组
  - 返回 Read 规则列表

- [ ] 3.4 实现 `write_claude_code_permissions()` 函数
  - 读取现有 settings.json (如果存在)
  - 更新 permissions.deny 数组
  - 保留其他设置
  - 使用原子写入
  - 确保有效的 JSON 格式

## 4. CLI 命令集成

- [ ] 4.1 在 `src/tool/command.rs` 添加 `SyncIgnore` 变体
  - 定义 `SyncIgnoreArgs` 结构体
  - 添加 `--from`, `--to`, `--input`, `--output`, `--bidirectional`, `--interactive`, `--dry-run`, `--verbose` 参数
  - 添加别名 `si`

- [ ] 4.2 在 `src/cli.rs` 添加处理函数
  - 在 `handle_tool_command` 中匹配 `SyncIgnore`
  - 调用 `sync_ignore::run()`

- [ ] 4.3 创建 `src/tool/sync_ignore.rs` 模块
  - 实现 `run()` 主函数
  - 实现参数解析和验证
  - 实现执行流程

## 5. 交互式模式

- [ ] 5.1 实现选择转换方向提示
  - 使用 `inquire::Select` 显示选项
  - Cursor → Claude Code
  - Claude Code → Cursor
  - 双向同步

- [ ] 5.2 实现输入文件提示
  - 允许用户输入文件路径
  - 提供默认值
  - 验证文件存在

- [ ] 5.3 实现预览和确认
  - 显示转换预览
  - 使用 `inquire::Confirm` 确认

- [ ] 5.4 实现 `interactive_mode()` 主函数
  - 协调所有交互步骤
  - 处理取消操作

## 6. 试运行和详细输出

- [ ] 6.1 实现试运行模式
  - 跳过实际文件写入
  - 显示所有更改
  - 显示源和目标路径

- [ ] 6.2 实现详细输出模式
  - 显示每个模式的转换
  - 显示所有警告
  - 显示跳过的规则

- [ ] 6.3 添加警告输出函数
  - 否定模式警告
  - 非 Read 规则警告
  - 文件权限警告

## 7. 双向同步

- [ ] 7.1 实现双向读取
  - 读取 `.cursorignore`
  - 读取 `.claude/settings.json`
  - 处理任一文件不存在的情况

- [ ] 7.2 实现双向转换
  - 转换 cursorignore → Claude Code
  - 转换 Claude Code → cursorignore

- [ ] 7.3 实现合并去重
  - 合并转换后的规则
  - 去除重复项
  - 保留两个来源的独特规则

- [ ] 7.4 实现双向写入
  - 写回 `.cursorignore`
  - 写回 `.claude/settings.json`

## 8. x 子命令集成

- [ ] 8.1 在 `src/x/claude_code/command.rs` 添加 `SyncIgnore` 子命令
  - 定义子命令参数
  - 添加别名 `si`
  - 默认目标为 Claude Code

- [ ] 8.2 实现 Claude Code 转发处理函数
  - 转发到 `sync_ignore::run_with_target()`
  - 预配置目标为 "claude-code"

- [ ] 8.3 在 `src/x/cursor/command.rs` 添加 `SyncIgnore` 子命令
  - 定义子命令参数
  - 添加别名 `si`
  - 默认目标为 Cursor

- [ ] 8.4 实现 Cursor 转发处理函数
  - 转发到 `sync_ignore::run_with_target()`
  - 预配置目标为 "cursor"

## 9. 国际化 (i18n)

- [ ] 9.1 在 `locales/app.yml` 添加 sync_ignore 键
  - 添加命令描述
  - 添加参数帮助文本
  - 添加交互式提示文本
  - 添加错误消息
  - 添加警告消息

- [ ] 9.2 在代码中使用 `t!()` 宏
  - 替换硬编码文本
  - 确保所有用户可见文本都支持 i18n

## 10. 错误处理

- [ ] 10.1 实现源文件不存在错误
  - 清晰的错误消息
  - 建议检查文件路径
  - 返回非零退出码

- [ ] 10.2 实现 JSON 解析错误处理
  - 显示解析错误详情
  - 指示 JSON 问题位置
  - 返回非零退出码

- [ ] 10.3 实现权限错误处理
  - 检查目录可写性
  - 显示权限问题
  - 返回非零退出码

- [ ] 10.4 实现通用错误处理
  - 使用 `anyhow::Context` 添加上下文
  - 确保所有错误都有清晰消息

## 11. 测试

- [ ] 11.1 创建 `tests/sync_ignore_tests.rs`
  - 添加基本转换测试
  - 添加边界情况测试
  - 添加错误处理测试

- [ ] 11.2 实现单元测试
  - 测试 `cursor_to_claude_code()`
  - 测试 `claude_code_to_cursor()`
  - 测试 `detect_format()`
  - 测试模式合并去重

- [ ] 11.3 实现集成测试
  - 测试完整转换流程
  - 测试交互式模式
  - 测试双向同步
  - 测试 x 子命令

- [ ] 11.4 添加临时目录测试
  - 使用 `tempfile::TempDir`
  - 避免污染工作目录
  - 测试后自动清理

## 12. 文档和验证

- [ ] 12.1 更新 CLI 帮助文本
  - 确保 `--help` 显示正确信息
  - 添加使用示例

- [ ] 12.2 运行 `just fmt` 检查格式
  - 修复所有格式问题

- [ ] 12.3 运行 `just lint` (clippy)
  - 修复所有警告
  - 确保 `-D warnings` 通过

- [ ] 12.4 运行 `just test`
  - 确保所有测试通过

- [ ] 12.5 手动测试
  - 测试 `llman tool sync-ignore --help`
  - 测试基本转换
  - 测试交互式模式
  - 测试 x 子命令
  - 测试试运行模式
  - 测试双向同步

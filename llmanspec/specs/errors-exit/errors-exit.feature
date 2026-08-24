# language: zh-CN
# capability: errors-exit
# purpose: 规范 llman CLI 的错误渲染与退出行为。
# scope: llmanspec/specs/errors-exit

功能: errors-exit

  @req:r22 @human
  场景: CLI 入口错误渲染
    - When a subcommand returns an error, the CLI MUST print a single user-visible error line to stderr and exit with code 1.

  @req:r53 @human
  场景: show --json 错误输出
    - When `sdd show` is called for a nonexistent spec with `--json`, the process MUST exit with code 1 and emit an Error on stderr (JSON-shaped error on stdout is not currently guaranteed).

  @req:r76 @human
  场景: 子命令错误处理
    - Command handlers MUST return Err on fatal errors. Interactive flows MAY print errors and exit themselves; recoverable issues MAY log to stderr without failing the command.
  @req:r22
  @executable
  场景: 子命令返回错误时打印单行错误并以退出码 1 退出
    假如 llman 二进制已构建
    当 在非交互终端运行 llman sdd show
    那么 退出码为 1
    那么 stderr 包含 Error

  @req:r53
  @executable
  场景: json-错误输出
    假如 llman 二进制已构建
    当 运行 llman sdd show nonexistent --type spec --output json
    那么 退出码为 1
    那么 stderr 包含 Error

  @req:r76
  @executable
  场景: 非交互终端下 sdd show 无参数时以退出码 1 退出
    假如 llman 二进制已构建
    当 在非交互终端运行 llman sdd show
    那么 退出码为 1
    那么 stderr 包含 Nothing to show


  @req:r76
  @executable
  场景: 查看不存在的 spec 时正常报错而非 panic
    假如 llman 二进制已构建
    当 运行 llman sdd show nonexistent-spec --type spec
    那么 退出码非零
    那么 stderr 包含 Error

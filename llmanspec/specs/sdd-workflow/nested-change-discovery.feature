# language: zh-CN
# 对应 spec: sdd-workflow r127/r128/r129 — 嵌套 change 递归发现、path 字段、
# max-scan-depth、graph 无分组节点、archive 扁平。
# 多数场景依赖 fixture 布局与 public discovery API，保持 fast mode（不标 @executable）；
# 由单元/集成测试在 apply 时覆盖；CLI 冒烟可在实现后补 @executable。
功能: 嵌套 change 目录发现与命令面

  @req:r127
  场景: 递归发现分组下的 proposal.md
    假如 llmanspec/changes/some_a/c0/proposal.md 存在且扁平同名不冲突
    当 运行 llman sdd list
    那么 输出包含叶子 id c0
    而且 不把 some_a 当作 change

  @req:r127
  场景: 同名叶子 id 发现即失败
    假如 llmanspec/changes/some_a/dup/proposal.md 与 llmanspec/changes/some_b/dup/proposal.md 同时存在
    当 运行 llman sdd list
    那么 退出码非零
    而且 stderr 包含冲突相对路径 some_a/dup
    而且 stderr 包含冲突相对路径 some_b/dup

  @req:r127
  场景: show 经 resolve 打开嵌套 change
    假如 llmanspec/changes/some_a/c0/proposal.md 存在
    当 运行 llman sdd show c0 --type change
    那么 退出码为零
    而且 输出可读到该 proposal 内容或 stage

  @req:r128
  场景: list JSON 含相对 changes 的 path
    假如 llmanspec/changes/some_a/c0/proposal.md 存在
    当 运行 llman sdd list --json
    那么 stdout 为合法 JSON
    而且 对应 change 条目含 path 值为 some_a/c0

  @req:r128
  场景: show JSON 含 path 且 id 为叶子名
    假如 llmanspec/changes/some_a/c0/proposal.md 存在
    当 运行 llman sdd show c0 --type change --output json
    那么 stdout 含 JSON 键 id
    而且 stdout 含 JSON 键 path
    而且 id 为 c0 且 path 为 some_a/c0

  @req:r128
  场景: max-scan-depth 限制发现深度
    假如 嵌套深度大于默认或指定上限的 change 仅在更深路径存在
    当 运行 llman sdd --max-scan-depth 1 list
    那么 不列出超出深度的叶子 id
    而且 当 N 小于 1 时非零退出

  @req:r129
  场景: graph 不把分组目录画成节点
    假如 llmanspec/changes/some_a/ 仅为分组且 some_a/c0/proposal.md 存在
    当 运行 llman sdd graph --format mermaid --scope active
    那么 输出含节点 c0
    而且 输出不含将 some_a 作为 change 节点

  @req:r129
  场景: archive 扁平移入 archive 目录
    假如 嵌套 change some_a/c0 已可归档
    当 运行 llman sdd change archive c0
    那么 文档位于 changes/archive/<date>-c0/
    而且 不保留 some_a 分组路径

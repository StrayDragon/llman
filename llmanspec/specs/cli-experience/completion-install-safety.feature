# language: zh-CN
# 对应 spec: cli-experience — self completion --shell 生成补全脚本；--install 写入标记块；
# 输出可复制；rc 写入前确认（非交互需 --yes）；幂等更新；不支持 shell 报错；
# PowerShell 写入目标 MUST 位于 home 下。
功能: shell 补全生成与 install 安全写入
  @req:r14
  场景: 生成 bash 补全
    假如 用户运行 llman self completion --shell bash
    当 命令执行
    而且 那么补全脚本写到 stdout
    而且 而且命令成功退出

  @req:r14
  场景: install 补全片段到 rc
    假如 用户运行 llman self completion --shell bash --install
    当 命令执行
    而且 那么bash rc/profile 文件中新增或更新标记补全块
    而且 而且命令成功退出

  @req:r14
  场景: install 输出打印实际片段
    假如 安装了一个补全片段
    当 命令执行
    而且 那么该片段写到 stdout

  @req:r14
  场景: 拒绝确认不产生副作用
    假如 用户拒绝确认提示
    当 命令执行
    而且 那么不修改任何 rc/profile 文件

  @req:r14
  场景: 非交互 install 未提供 --yes 时拒绝
    假如 命令在非交互环境运行且含 --install 但无 --yes
    当 命令执行
    而且 那么非零退出
    而且 而且不修改任何 rc/profile 文件

  @req:r14
  场景: 非交互 install 提供 --yes 时直接写入
    假如 命令在非交互环境运行且含 --install --yes
    当 命令执行
    而且 那么补全块被安装/更新
    而且 而且命令成功退出

  @req:r14
  场景: 已存在块时原地更新不重复
    假如 shell rc/profile 已含标记 llman 补全块
    当 执行 install
    而且 那么该块被替换为最新片段
    而且 而且不新增重复块

  @req:r14
  场景: 不支持的 shell 值报错
    假如 用户传 --shell {unsupported}
    当 命令执行
    而且 那么报告支持的 shell 列表
    而且 而且以失败退出

  @req:r14
  场景: PowerShell 拒绝 home 外的 profile
    假如 PROFILE={external_path} 且该路径不在用户 home 下
    当 用户运行 llman self completion --shell powershell --yes
    而且 那么命令失败并拒绝写入
    而且 而且文件系统中未写入补全块

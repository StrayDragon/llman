# language: zh-CN
# managed by llman sdd partition-migrate
功能: cli-experience

  @req:r1
  场景: 生成 bash 补全
    假如 用户运行 llman self completion --shell bash
    当 命令执行
    那么 补全脚本写到 stdout
    而且 命令成功退出

  @req:r1
  场景: install 补全片段到 rc
    假如 用户运行 llman self completion --shell bash --install
    当 命令执行
    那么 bash rc/profile 文件中新增或更新标记补全块
    而且 命令成功退出

  @req:r1
  场景: install 输出打印实际片段
    假如 安装了一个补全片段
    当 命令执行
    那么 该片段写到 stdout

  @req:r1
  场景: 拒绝确认不产生副作用
    假如 用户拒绝确认提示
    当 命令执行
    那么 不修改任何 rc/profile 文件

  @req:r1
  场景: 非交互 install 未提供 --yes 时拒绝
    假如 命令在非交互环境运行且含 --install 但无 --yes
    当 命令执行
    那么 非零退出
    而且 不修改任何 rc/profile 文件

  @req:r1
  场景: 非交互 install 提供 --yes 时直接写入
    假如 命令在非交互环境运行且含 --install --yes
    当 命令执行
    那么 补全块被安装/更新
    而且 命令成功退出

  @req:r1
  场景: 已存在块时原地更新不重复
    假如 shell rc/profile 已含标记 llman 补全块
    当 执行 install
    那么 该块被替换为最新片段
    而且 不新增重复块

  @req:r1
  场景: 不支持的 shell 值报错
    假如 用户传 --shell {unsupported}
    当 命令执行
    那么 报告支持的 shell 列表
    而且 以失败退出

  @req:r1
  场景: PowerShell 拒绝 home 外的 profile
    假如 PROFILE={external_path} 且该路径不在用户 home 下
    当 用户运行 llman self completion --shell powershell --yes
    那么 命令失败并拒绝写入
    而且 文件系统中未写入补全块

  @req:r1
  场景: 本地化提示含内联格式化
    假如 某命令提示用户或打印状态头
    当 输出消息
    那么 主要文本从 locales/app.yml 解析
    而且 内联标记（emoji/bullet/分隔符）作为字面量嵌入

  @req:r1
  场景: 生成内容可含非本地化硬编码文本
    假如 某命令输出生成内容（如导出 markdown 或文件名标签）
    当 输出
    那么 可含未本地化的硬编码文本

  @req:r1
  场景: CLI 启动 locale 为 en
    假如 CLI 启动
    当 运行时初始化
    那么 locale 设为 en
    而且 本地化键从 locales/app.yml 解析

  @req:r1
  场景: 执行失败错误写到 stderr
    假如 某命令执行中失败
    当 输出错误
    那么 面向用户的错误写到 stderr

  @req:r1
  场景: 进度与结果写到 stdout
    假如 某命令报告进度或结果
    当 输出
    那么 消息写到 stdout

  @req:r1
  场景: 单行标签格式一致
    假如 打印单行状态或标签
    当 输出
    那么 使用一致格式
    而且 不混用无关前缀

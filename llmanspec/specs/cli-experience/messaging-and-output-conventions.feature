# language: zh-CN
# 对应 spec: cli-experience — 运行时提示/状态/错误 MUST 优先用 t! 本地化键；locale 固定英文；
# 正常输出与交互提示到 stdout，错误到 stderr；单行消息使用一致前缀。
功能: 本地化消息与 stdout/stderr 约定
  @req:r43
  场景: 本地化提示含内联格式化
    假如 某命令提示用户或打印状态头
    当 输出消息
    而且 那么主要文本从 locales/app.yml 解析
    而且 而且内联标记（emoji/bullet/分隔符）作为字面量嵌入

  @req:r43
  场景: 生成内容可含非本地化硬编码文本
    假如 某命令输出生成内容（如导出 markdown 或文件名标签）
    当 输出
    而且 那么可含未本地化的硬编码文本

  @req:r43
  场景: CLI 启动 locale 为 en
    假如 CLI 启动
    当 运行时初始化
    而且 那么locale 设为 en
    而且 而且本地化键从 locales/app.yml 解析

  @req:r43
  场景: 执行失败错误写到 stderr
    假如 某命令执行中失败
    当 输出错误
    而且 那么面向用户的错误写到 stderr

  @req:r43
  场景: 进度与结果写到 stdout
    假如 某命令报告进度或结果
    当 输出
    而且 那么消息写到 stdout

  @req:r43
  场景: 单行标签格式一致
    假如 打印单行状态或标签
    当 输出
    而且 那么使用一致格式
    而且 而且不混用无关前缀

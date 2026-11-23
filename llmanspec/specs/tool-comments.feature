# language: zh-CN
# capability: tool-comments
# purpose: 规范 `llman tool comments` 命令组（overview 文件头注释纵览，clean-useless 无用注释清理入口）。
# scope: llmanspec/specs/tool-comments.feature

功能: tool-comments

  @req:r143
  规则: comments 组与 overview TARGET 必填
    `llman tool comments` MUST 作为 `llman tool` 子命令出现在 `--help` 中，并暴露 `overview` 与 `clean-useless` 两个子命令。`llman tool comments overview` MUST 要求至少一个 TARGET（目录、文件或 glob）；缺省 TARGET 时 MUST 非零退出且不扫描 cwd。

    场景: comments-group-help-surface
      假如 llman 二进制已构建
      当 运行 llman tool comments --help
      那么 退出码为零
      那么 stdout 包含 overview
      那么 stdout 包含 clean-useless

    场景: comments-overview-help-surface
      假如 llman 二进制已构建
      当 运行 llman tool comments overview --help
      那么 退出码为零
      那么 stdout 包含 overview

    场景: comments-overview-requires-target
      假如 llman 二进制已构建
      当 运行 llman tool comments overview
      那么 退出码非零

  @req:r144
  规则: overview TARGET 解析、gitignore 与零命中
    每个 TARGET MUST 按顺序判定：路径存在且为文件则始终处理（即使被 gitignore）；路径存在且为目录则递归该树并尊重 gitignore；否则若含 glob 元字符 `*?[{` 则按 globset 展开（MUST 支持 `**` 与 `{a,b}`）且命中集尊重 gitignore。多 TARGET MUST 并集后按相对 cwd 的路径排序去重。TARGET 既不是存在的路径、glob 又零命中时 MUST 非零退出并向 stderr 打印单行错误。目录存在但没有可抽取的头注释时 MUST 以退出码 0 写出空 stdout。

    场景: comments-overview-empty-dir-succeeds
      假如 含注释 python 文件与空目录的临时项目
      当 运行 llman tool comments overview empty-junk
      那么 退出码为零
      那么 stdout 不含 code.py

    场景: comments-overview-glob-zero-match-fails
      假如 临时工作目录
      当 运行 llman tool comments overview no-such-glob-xyzzy-*
      那么 退出码非零
      那么 stderr 包含 Error

    场景: comments-overview-dir-honors-gitignore
      假如 含头注释文件与 gitignore 的临时项目
      当 运行 llman tool comments overview .
      那么 退出码为零
      那么 stdout 包含 keep.py
      那么 stdout 不含 skip.py

    场景: comments-overview-explicit-file-bypasses-gitignore
      假如 含头注释文件与 gitignore 的临时项目
      当 运行 llman tool comments overview skip.py
      那么 退出码为零
      那么 stdout 包含 skip.py

  @req:r145
  规则: 文件头第一段嗅探与 TSV 输出
    抽取 MUST 跳过首行 shebang，按文件头内容嗅探 `#`、`//`、`--`、`/* */`、`"""`/`'''`，在空行或注释结束处停止（只要第一段）。无头注释、含 NUL 或非 UTF-8 的文件 MUST 静默跳过且不使命令失败。有描述时 stdout MUST 为 TSV 行 `相对路径<TAB>第一段`；描述若以文件 basename 开头 MUST 剥掉 basename 与分隔符 `[-:=|]`。MUST NOT 使用 tree-sitter，MUST NOT 扫描文件中段注释，MUST NOT 向 stdout 打印非 TSV 横幅。

    场景: comments-overview-extracts-leading-hash
      假如 含注释 python 文件与空目录的临时项目
      当 运行 llman tool comments overview code.py
      那么 退出码为零
      那么 stdout 包含 code.py
      那么 stdout 包含 top comment

  @req:r146
  规则: clean-useless 规范入口与旧名 alias
    `llman tool comments clean-useless` MUST 与既有 `llman tool clean-useless-comments` 使用同一清理行为（相同旗标与 runner）。`clean-useless-comments` 与 `cuc` MUST 仍可调用。`llman tool --help` MUST 列出 `comments` 且 MUST NOT 把 `clean-useless-comments` 作为一级可见子命令。

    场景: comments-clean-useless-help-surface
      假如 llman 二进制已构建
      当 运行 llman tool comments clean-useless --help
      那么 退出码为零
      那么 stdout 包含 clean-useless

    场景: clean-useless-comments-hidden-alias-help
      假如 llman 二进制已构建
      当 运行 llman tool clean-useless-comments --help
      那么 退出码为零

    场景: tool-help-lists-comments-not-old-clean
      假如 llman 二进制已构建
      当 运行 llman tool --help
      那么 退出码为零
      那么 stdout 包含 comments
      那么 stdout 不含 clean-useless-comments

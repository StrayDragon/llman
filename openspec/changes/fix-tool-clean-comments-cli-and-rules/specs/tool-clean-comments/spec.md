## ADDED Requirements
### Requirement: 非文件输入必须被显式处理
当用户通过 `files...` 显式传入路径时，clean-comments processor MUST 显式处理目录与其它非普通文件路径。它 MUST NOT 将其当作文件读取，并 MUST 输出清晰的非致命提示，同时继续处理其它输入。

#### Scenario: 目录输入被跳过
- **WHEN** 用户运行 `llman tool clean-useless-comments path/to/dir`
- **THEN** 工具提示该目录输入被跳过（或需要显式展开），且不会把它当成文件读取失败

### Requirement: doc comment 开关语义必须一致
doc comment 移除开关（`docstrings`、`jsdoc`、`doc-comments`、`godoc`）MUST 与其它注释开关保持一致语义：`true` 启用移除、`false` 禁用移除、`None` 默认禁用移除（即保留 doc comments）。

#### Scenario: docstrings=true 启用移除
- **WHEN** 配置对某语言设置 `docstrings: true` 且文件包含 doc comments
- **THEN** doc comments 在规则判断中可被移除（满足其它条件时）

#### Scenario: 未配置 docstrings 默认保留
- **WHEN** 配置未指定 doc comment 开关
- **THEN** doc comments 被保留

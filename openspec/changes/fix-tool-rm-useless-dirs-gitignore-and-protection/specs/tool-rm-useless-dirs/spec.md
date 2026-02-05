## ADDED Requirements
### Requirement: 默认 gitignore 必须相对扫描目标解析
当提供扫描目标路径时，工具 MUST 将默认 `.gitignore` 解析为 `<target>/.gitignore`（当其存在且为文件）。工具 MUST NOT 对其它 target 隐式使用当前工作目录的 `.gitignore`。

#### Scenario: 非 CWD target 使用自己的 gitignore
- **WHEN** 用户运行 `llman tool rm-useless-dirs /tmp/project` 且未传 `--gitignore`
- **THEN** 若 `/tmp/project/.gitignore` 存在，则工具使用它

### Requirement: protected 名称在整棵路径树中生效
protected 目录 basenames MUST 在整个扫描树中生效。工具 MUST NOT 遍历进入任何路径中包含 protected 组件的目录。

#### Scenario: 扫描遇到 protected 组件
- **WHEN** 扫描遇到 `some/.git/objects`
- **THEN** 工具不进入 `some/.git` 子树，且不会删除其下任何内容

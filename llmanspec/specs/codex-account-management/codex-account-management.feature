# language: zh-CN
# capability: codex-account-management
# purpose: 规范 `llman x codex` 账户管理：编辑器命令参数支持与账户配置行为。
# scope: llmanspec/specs/codex-account-management

功能: codex-account-management

  @req:r15 @human
  场景: 编辑器参数支持与 provider 配置 upsert
    - 对应 spec: codex-account-management — 编辑器命令 MUST 支持 $VISUAL/$EDITOR 含参数； 切换组时 MUST 将 provider 配置 upsert 到 ~/.codex/config.toml 并设置顶层 model_provider， 支持 override_name 覆盖 effective_name。

  @req:r44 @human
  场景: 环境变量安全、交互导入与命令透传
    - 对应 spec: codex-account-management — env 注入 MUST 拒绝危险键（LD_PRELOAD/LD_LIBRARY_PATH/ DYLD_*/PATH 及大小写变体），拒绝时不启动 codex；import 交互式创建 provider； 主命令/run 支持 -- 透传；account 提供 edit 与 import。

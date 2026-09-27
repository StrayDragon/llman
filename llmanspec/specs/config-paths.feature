# language: zh-CN
# capability: config-paths
# purpose: 规范配置目录解析、安全 IO 与 context 索引路径。
# scope: llmanspec/specs/config-paths.feature

功能: config-paths

  @req:r17
  规则: 配置目录按优先级解析
    CLI MUST resolve the config directory by priority: CLI `--config-dir` > `LLMAN_CONFIG_DIR` > default `~/.config/llman`. The resolved value MUST be assigned to `LLMAN_CONFIG_DIR`. Resolution MUST NOT create directories.

    场景: config-dir-priority-cli-flag-over-env
      假如 llman 二进制已构建
      当 同时以 -C 与环境变量运行 llman --print-config-dir-path
      那么 退出码为零
      那么 stdout 为 -C 指定目录

    场景: config-dir-priority-env-over-default
      当 以环境变量 LLMAN_CONFIG_DIR 指向临时目录运行 llman --print-config-dir-path
      那么 退出码为零
      那么 stdout 为环境变量指定目录

    场景: config-dir-default-under-home
      当 不带任何配置来源运行 llman --print-config-dir-path
      那么 退出码为零
      那么 stdout 为 HOME 下的默认配置目录
  @req:r46
  规则: context 索引写入 .context 目录
    The CLI MUST store the embedding index under `<config-dir>/.context/` (overridable by `LLMAN_CONTEXT_DIR`), containing metadata.toml, vectors.bin, specs.json, and optional chunks.json as specified for context indexing.
  @req:r72
  规则: 非法路径报错与安全 IO 边界
    The resolver MUST error on empty/whitespace CLI/env paths without creating directories. `read_with_max_size` MUST reject oversized files (default 10 MiB). Atomic writes MUST delete a symlink target first rather than following it.

    场景: empty-env-config-path-errors
      当 以环境变量 LLMAN_CONFIG_DIR 为空串运行 llman --print-config-dir-path
      那么 退出码非零
      那么 stderr 包含 only whitespace

    场景: whitespace-cli-config-path-errors
      当 以 -C 空白路径运行 llman --print-config-dir-path
      那么 退出码非零
      那么 stderr 包含 Invalid config directory
  @req:r4
  规则: tilde 展开与开发仓库守卫
    When `--config-dir` or `LLMAN_CONFIG_DIR` starts with `~`, it MUST expand to the user home directory. Running inside the llman development repository without an explicit override MUST error and require an explicit config dir.

    场景: tilde-env-path-expands-to-home
      当 以环境变量 LLMAN_CONFIG_DIR 以 ~ 开头运行 llman --print-config-dir-path
      那么 退出码为零
      那么 stdout 为 HOME 下的展开路径

# design — add-archive-freeze-list

## 背景

`freezed_changes.7z.archived` 是不透明的 7z 冷备份归档，没有独立的 manifest。
代码库此前只知道它是否存在（`has_frozen_archive` 布尔），无法枚举内容。validate
的 INFO 提示引导用户运行 `archive freeze --list`，但该标志不存在。

## 决策 1：用 `--list` 标志，而非 `archive list` 子命令

- **选择**：`llman sdd archive freeze --list`（布尔标志短路）。
- **理由**：
  - 提示文案已经写死 `archive freeze --list`，遵循它实现零 locale 改动、零
    迁移成本。
  - 列出与冻结是同一资源的两种视角（同一 `freezed_changes.7z.archived`），
    语义内聚于 `freeze` 子命令，比新增 `archive list` 更连贯。
  - `--dry-run` 已开创"freeze 子命令的只读模式"先例，`--list` 与之一致。
- **备选（放弃）**：新增 `ArchiveSubcommand::List`。会与既有文案不符，且需要
  改 locale + 文档，违背"最小化"。

## 决策 2：复用 thaw 的 7z-decompress-to-tempdir 模式

- **选择**：`list_frozen()` 调用 `sevenz_rust2::decompress_file(&freeze_file, tempdir)`
  后枚举顶层目录，与 `run_thaw_with_root`（selective thaw 路径，freeze.rs:143-161）
  完全同构。
- **理由**：
  - `sevenz_rust2` 没有公开稳定的"只列目录不解压"API；decompress 是已验证的路径。
  - tempdir 自动清理，列表操作不产生任何持久副作用（只读语义）。
  - 按顶层目录名排序输出，保证跨运行稳定（7z 内部顺序不保证）。
- **备选（放弃）**：用 `7z` 外部子进程（用户报告中的 `7z l`）。会引入外部二进制
  依赖，跨平台不可靠。

## 决策 3：短路位置在 `archive_dir.exists()` 之后、候选选择之前

`run_freeze_with_root` 开头先校验 `archive_dir` 存在。`--list` 在该校验**之后**
短路：
- 保留"没有 archive 目录 → 报错"的一致性（与 freeze/thaw 一致）。
- 列表只依赖 `freeze_file`，不依赖磁盘上的候选目录，所以无需 `select_candidates`。

## 非目标

- 不改 locale 文案（提示已是 truthful，待 `--list` 上线即自洽）。
- 不为 `thaw` 增加 `--list`（thaw 的 `--change` 已能选择性恢复，语义不同）。
- 不引入 freeze manifest 文件（7z 本身即 SSOT，维护双源更易出错）。

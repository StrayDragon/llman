# tasks — add-archive-freeze-list

> 追溯提案：代码实现先于本提案（commit 0814467），任务用于核对实施完整性与跑门禁。

## Tasks

- [x] 在 `src/sdd/command.rs` 的 `ArchiveSubcommand::Freeze` 增加 `--list` 布尔标志，并在 dispatch 中透传到 `FreezeArgs.list`
- [x] 在 `src/sdd/change/freeze.rs` 的 `FreezeArgs` 增加 `list: bool` 字段
- [x] 实现 `list_frozen(archive_dir)`：复用 thaw 的 `sevenz_rust2::decompress_file` → tempdir 模式，枚举顶层 `YYYY-MM-DD-id` 目录并排序输出；无归档文件时提示 "No freeze archive found"
- [x] 在 `run_freeze_with_root` 中 `archive_dir.exists()` 校验之后短路到 `list_frozen`（只读，不改文件系统）
- [x] 在 `tests/sdd_integration_tests.rs` 新增 3 个集成测试：枚举内容 / 无归档文件提示 / 列举不产生 `.thawed` 副作用
- [x] `just fmt` + `just lint`（clippy `-D warnings`）通过
- [x] `just check` 全量测试通过（505/505）
- [x] `llman sdd validate add-archive-freeze-list --strict --no-interactive` 通过
- [x] `llman sdd change checkpoint add-archive-freeze-list` 通过（干净工作区 + 门禁）
- [x] `llman-sdd-verify add-archive-freeze-list` 全绿（validate --strict + BDD full mode + 505 集成测试 + dualWriteCount=0；无 CRITICAL）
- [x] `llman sdd change archive add-archive-freeze-list`（docs-only，BDD-on）

# Tasks

垂直切片：每个 task 打穿「实现 → 测试 → 可独立验证」。
测试 seam：CLI 子进程（`tests/bdd/steps/shared.ts` 的 `运行 llman {args:rest}`，MUST 复用）+ `extract_header` 纯函数。gitignore/glob fixture 用 TempDir，MUST 写在 TempDir 内。

## T1: 头注释纯函数

- [x] `extract_header(source, basename) -> Option<String>`：shebang 跳过、五种风格、第一段、basename 剥离
- [x] 单元测试（模块内 `#[cfg(test)]`）：`#` / `//` / `--` / `/* */` / `"""`、空行截断、无头 → None、`file.py: desc` 剥前缀

## T2: comments 组 CLI + overview TARGET [blocked-by: T1]

- [x] clap：`llman tool comments overview <TARGET>...`（至少一个 TARGET）；`llman tool comments clean-useless` 复用既有 args/runner
- [x] 顶层 `clean-useless-comments` / `cuc` 为 hidden alias，同一 `clean_comments::run`
- [x] overview：文件 / 目录 / glob（`globset`，`**` 与 `{a,b}`）；目录与 glob 尊重 gitignore；显式文件始终处理
- [x] 零命中非零退出（stderr 单行错误）；目录无头注释 → 空 stdout、exit 0
- [x] stdout 仅 TSV，路径相对 cwd，排序去重；二进制/非法 UTF-8 跳过
- [x] i18n 键（`locales/app.yml`）
- [x] `tests/it` 新模块并注册：显式 gitignored 文件仍输出；glob 零命中非零；目录空头 exit 0；`comments clean-useless --help` 与 hidden 旧名 `--help` 均可

## T3: BDD 绑定 [blocked-by: T2]

- [x] `tests/bdd/steps/tools.ts`：gitignore 头注释 fixture
- [x] `tests/bdd/run.test.ts` allowlist 纳入 `tool-comments.feature`
- [x] 绑定 help、显式文件抽取、gitignore、glob 零命中、clean-useless help 场景

## T4: 文档 [blocked-by: T2]

- [x] `just readme` 重生成命令表
- [x] 本仓库 `AGENTS.md` 加一行：`llman tool comments overview 'scripts/**/*.{sh,py,lua,js,mjs,ts}'`（不列清单）

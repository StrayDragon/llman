---
depends_on: []
branch: sdd/support-nested-change-dir-groups
base_sha: 738d1027c6a601bd007860faf63255c0032671fd
checkpointed: false
---

# 支持 changes 下按目录分组（递归发现 proposal.md）

> Open Questions 已拍板（见「已决议」）。Seam A–F 已确认。

## Why

用户会在 `llmanspec/changes/` 下用中间目录给大量候选提案分组，例如：

```
llmanspec/changes/
  some_a/{c0,c1,c2}/proposal.md
  some_b/{c10,c11,c12}/proposal.md
  some_c/{c20,c21,c22}/proposal.md
```

当前 `list_changes` 只扫 **直子目录**；几乎所有命令用 `changes.join(id)` 扁平解析。分组布局下嵌套 change **不可见、不可 show/validate/start**。需要把「含 `proposal.md` 的目录」定义为 change，并递归发现。

## What Changes

### 发现与身份

- **Change 定义**：`llmanspec/changes/` 下（跳过 `archive/` 与 `.` 开头）在扫描深度内、自身含 `proposal.md` 的目录。
- **Change id**：仍为**叶子目录名**（`c0`），保持 `validate_sdd_id`（禁止 `/`）。分组目录只是组织，不是 id 的一部分。
- **唯一性**：全库叶子 id MUST 唯一。**发现即 ERROR**（`list_changes` / resolve 失败），错误信息 MUST 列出冲突的相对路径，便于 agent 处理；不得静默丢条目。
- **路径解析 SSOT**：新增集中 `resolve_change_dir(root, id) -> PathBuf`（或等价）；禁止各命令自行 `changes.join(id)`。
- **跳过规则**：`archive/` 不递归进 active；无 `proposal.md` 的中间目录不是 change；**不跟随符号链接**。
- **扫描深度**：默认 **8**（相对 `changes/`：深度 1 = 直子目录）。**仅 CLI 可调**，不写 `config.yaml` 字段（见下）。

### 扫描深度（决议 #5，CLI-only）

发现是全命令共享 SSOT：深度 MUST 由 **`llman sdd` 顶层** CLI 传入，使 `list`/`show`/`start`/`validate`/`graph` 等读到同一有效值。

| 层 | 机制 | 作用 |
|----|------|------|
| 默认 | `8` | 未传 flag 时 |
| CLI | `llman sdd --max-scan-depth <N> …`（顶层） | 覆盖默认；本次进程内所有发现共用 |

不做 `llmanspec/config.yaml` 深度字段。`N < 1` 非法。

### 命令面

| 命令 / 表面 | 行为 |
|-------------|------|
| **发现 SSOT** | 递归找 `proposal.md`；尊重 max depth；冲突 Err + 路径列表 |
| **`list`** | 人读默认显示叶子 id；**JSON 增加 `path`**（相对 `llmanspec/changes/`） |
| **`status`** | 经 `resolve_change_dir` |
| **`show <id>`** | resolve；人读/JSON 含 `path`；id 仍为叶子名 |
| **`validate`** | 递归发现 + resolve |
| **`graph`** | 删除 partial-node 启发式；分组目录不成节点 |
| **`change new`** | **不改**（无 `--group`） |
| **lifecycle** | start/attach/checkpoint/diff/finalize/archive 经 resolve；分支名 `sdd/<leaf-id>` |
| **archive** | 扁平移入 `archive/<date>-<id>/`；不保留分组路径 |

### Non-goals

- 不把分组路径编进 change id；不改 specs 发现模型；不做 `--group`；不做 archive `former_path`。

## Capabilities

- `sdd-workflow`：嵌套 change 发现、深度 CLI、list/show `path`、graph 无分组节点、lifecycle resolve、冲突提示。
- `cli`（若顶层 `SddArgs` flag 归 CLI 壳）：`--max-scan-depth` 解析与非法值。

## Impact

- 代码：`discovery.rs` SSOT；收敛 `changes.join(id)`；`SddArgs` 增 flag。
- 合约：扩 `sdd-workflow`（+ 必要时 `cli`）。
- 兼容：扁平不变；JSON `path` 附加字段。

## 已决议

1. list/show JSON `path`：加。
2. graph partial node：删。
3. archive：不保留分组/`former_path`。
4. `change new --group`：不处理。
5. 深度默认 8；仅顶层 `--max-scan-depth`；无 config。
6. 同名冲突：发现即 ERROR + 冲突相对路径。

## 测试边界（已确认）

- A `list` / `--json` · B `show` · C `graph` · D 发现冲突 · E `--max-scan-depth` · F start/archive 抽测

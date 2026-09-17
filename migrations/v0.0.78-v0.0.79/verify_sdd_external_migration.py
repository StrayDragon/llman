#!/usr/bin/env python3
"""One-shot environment check: external sdd delegation (v0.0.78 -> v0.0.79).

v0.0.79 removed the built-in sdd subsystem, so there is no in-repo data to
transform. This migration therefore ships a check-only script (trivially
dry-run and idempotent): it verifies the delegation chain is ready and prints
a manual-steps checklist for anything it cannot fix. It never writes files.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys

CARGO_BIN = os.path.expanduser("~/.cargo/bin")
CARGO_RESIDUE = ("llman-sdd", "llmanspec")
BUN_FALLBACK = os.path.expanduser("~/.bun/bin/llman-sdd")
MANUAL_STEPS = (
    "安装外部 CLI: bun/pnpm/npm i -g @llman-sdd/cli",
    "删除旧编译产物: rm -f ~/.cargo/bin/llman-sdd ~/.cargo/bin/llmanspec",
    "在含 llmanspec/ 的项目中运行 llman-sdd validate 验证数据目录",
)


def find_llman_sdd() -> str | None:
    exe = shutil.which("llman-sdd")
    if exe:
        return exe
    if os.path.isfile(BUN_FALLBACK) and os.access(BUN_FALLBACK, os.X_OK):
        return BUN_FALLBACK
    return None


def main() -> int:
    checks: list[tuple[bool, str]] = []

    hub = shutil.which("llman")
    checks.append((hub is not None, "llman hub 在 PATH 上" + (f" ({hub})" if hub else "")))

    plugin = find_llman_sdd()
    checks.append(
        (plugin is not None, "llman-sdd 可发现（PATH 或 ~/.bun/bin 回退）" + (f" ({plugin})" if plugin else ""))
    )

    delegation_ok = False
    if hub and plugin:
        proc = subprocess.run(
            ["llman", "sdd", "--version"], capture_output=True, text=True, timeout=120
        )
        delegation_ok = proc.returncode == 0
        detail = f"exit={proc.returncode} output={proc.stdout.strip() or proc.stderr.strip()!r}"
        checks.append((delegation_ok, f"llman sdd --version 端到端委托成功 ({detail})"))
    else:
        checks.append((False, "llman sdd --version 端到端委托成功 (前置条件未满足，跳过)"))

    residue = [n for n in CARGO_RESIDUE if os.path.exists(os.path.join(CARGO_BIN, n))]
    checks.append(
        (not residue, "~/.cargo/bin 无旧编译产物残留" + (f": {', '.join(residue)}" if residue else ""))
    )

    for ok, message in checks:
        print(f"{'PASS' if ok else 'FAIL'}  {message}")

    if all(ok for ok, _ in checks):
        print("\n✅ 迁移就绪：外部 sdd 委托链路可用（no-op，未修改任何文件）")
        return 0

    print("\n❌ 未就绪——人工处理清单（脚本不会自动修改）：")
    for step in MANUAL_STEPS:
        print(f"  - {step}")
    return 1


if __name__ == "__main__":
    sys.exit(main())

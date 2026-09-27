// Shared steps + harness helpers: resolve the built llman binary and run it
// as a hermetic subprocess (isolated HOME / LLMAN_CONFIG_DIR, controlled
// PATH — host plugin installs can never answer), mirroring the isolation
// contract of tests/it/external_subcommand.rs.

import { spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

import { bdd, type TestContext } from '../runner.ts';

export const REPO_ROOT = join(import.meta.dir, '..', '..', '..');
export const SYSTEM_PATH_DIRS = '/usr/bin:/bin';

export function tempDir(prefix = 'llman-bdd-'): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

export function llmanBin(): string {
  const override = process.env.LLMAN_BDD_BIN;
  if (override) return override;
  for (const profile of ['debug', 'release']) {
    const candidate = join(REPO_ROOT, 'target', profile, 'llman');
    if (existsSync(candidate)) return candidate;
  }
  throw new Error('llman binary not found; run `just build` (or set LLMAN_BDD_BIN)');
}

export interface RunResult {
  code: number;
  stdout: string;
  stderr: string;
}

export function runLlman(ctx: TestContext, args: string[], overrides?: { configDir?: string; cwd?: string; extraEnv?: Record<string, string> }): RunResult {
  const pluginDir = ctx.fixtures['插件目录'] as string | undefined;
  const configDir = overrides?.configDir ?? (ctx.fixtures['配置目录'] as string | undefined) ?? tempDir();
  const pathValue = [pluginDir, SYSTEM_PATH_DIRS].filter(Boolean).join(':');
  const result = spawnSync(llmanBin(), args, {
    cwd: overrides?.cwd ?? tempDir(),
    env: {
      PATH: pathValue,
      HOME: tempDir(),
      LLMAN_CONFIG_DIR: configDir,
      ...overrides?.extraEnv,
    },
    encoding: 'utf8',
  });
  const out: RunResult = {
    code: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
  ctx.fixtures['上次运行'] = out;
  return out;
}

export function lastRun(ctx: TestContext): RunResult {
  const run = ctx.fixtures['上次运行'] as RunResult | undefined;
  if (!run) throw new Error('no llman run recorded yet');
  return run;
}

// ---------------------------------------------------------------------------
// Generic steps shared by every bound feature
// ---------------------------------------------------------------------------

bdd.given('llman 二进制已构建', () => {
  llmanBin();
});

bdd.when('运行 llman {args:rest}', (ctx, params) => {
  runLlman(ctx, params.args.trim().split(/\s+/));
});

bdd.when('在非交互终端运行 llman {args:rest}', (ctx, params) => {
  runLlman(ctx, params.args.trim().split(/\s+/));
});

bdd.then('退出码为零', (ctx) => {
  if (lastRun(ctx).code !== 0) {
    throw new Error(`expected exit 0, got ${lastRun(ctx).code}\nstderr: ${lastRun(ctx).stderr}`);
  }
});

bdd.then('退出码为 {code:d}', (ctx, params) => {
  if (lastRun(ctx).code !== Number(params.code)) {
    throw new Error(`expected exit ${params.code}, got ${lastRun(ctx).code}\nstderr: ${lastRun(ctx).stderr}`);
  }
});

bdd.then('退出码非零', (ctx) => {
  if (lastRun(ctx).code === 0) throw new Error('expected non-zero exit, got 0');
});

bdd.then('stderr 包含 {text:rest}', (ctx, params) => {
  if (!lastRun(ctx).stderr.includes(params.text)) {
    throw new Error(`stderr does not contain "${params.text}"\nstderr: ${lastRun(ctx).stderr}`);
  }
});

bdd.then('stderr 不含 {text:rest}', (ctx, params) => {
  if (lastRun(ctx).stderr.includes(params.text)) {
    throw new Error(`stderr unexpectedly contains "${params.text}"\nstderr: ${lastRun(ctx).stderr}`);
  }
});

bdd.then('stdout 包含 {text:rest}', (ctx, params) => {
  if (!lastRun(ctx).stdout.includes(params.text)) {
    throw new Error(`stdout does not contain "${params.text}"\nstdout: ${lastRun(ctx).stdout}`);
  }
});

bdd.then('stdout 不含 {text:rest}', (ctx, params) => {
  if (lastRun(ctx).stdout.includes(params.text)) {
    throw new Error(`stdout unexpectedly contains "${params.text}"\nstdout: ${lastRun(ctx).stdout}`);
  }
});

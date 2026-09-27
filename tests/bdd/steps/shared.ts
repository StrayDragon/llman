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
  home: string;
}

export function runLlman(
  ctx: TestContext,
  args: string[],
  overrides?: { configDir?: string; cwd?: string; home?: string; omitConfigEnv?: boolean; extraEnv?: Record<string, string> },
): RunResult {
  const pluginDir = ctx.fixtures['插件目录'] as string | undefined;
  const home = overrides?.home ?? tempDir();
  const configDir = overrides?.configDir ?? (ctx.fixtures['配置目录'] as string | undefined) ?? tempDir();
  const pathValue = [pluginDir, SYSTEM_PATH_DIRS].filter(Boolean).join(':');
  const env: Record<string, string> = {
    PATH: pathValue,
    HOME: home,
    ...(overrides?.omitConfigEnv ? {} : { LLMAN_CONFIG_DIR: configDir }),
    ...overrides?.extraEnv,
  };
  const result = spawnSync(llmanBin(), args, {
    cwd: overrides?.cwd ?? (ctx.fixtures['工作目录'] as string | undefined) ?? tempDir(),
    env,
    encoding: 'utf8',
  });
  const out: RunResult = {
    code: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    home,
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

bdd.given('临时工作目录', (ctx) => {
  ctx.fixtures['工作目录'] = tempDir('llman-bdd-work-');
  ctx.fixtures['sdd 项目'] = ctx.fixtures['工作目录'];
});

bdd.when('运行 llman {args:rest}', (ctx, params) => {
  runLlman(ctx, params.args.trim().split(/\s+/));
});

bdd.when('在非交互终端运行 llman {args:rest}', (ctx, params) => {
  runLlman(ctx, params.args.trim().split(/\s+/));
});

bdd.then('退出码为零', (ctx) => {
  if (lastRun(ctx).code !== 0) {
    throw new Error(`expected exit 0, got ${lastRun(ctx).code}\nstderr: ${lastRun(ctx).stderr}\nstdout: ${lastRun(ctx).stdout}`);
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

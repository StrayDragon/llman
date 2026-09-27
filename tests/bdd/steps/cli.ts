// Steps for cli.feature r56 (external subcommand delegation): fake llman-*
// plugins as executable shell fixtures in a temp dir prepended to the child
// PATH — the same seam as tests/it/external_subcommand.rs.

import { chmodSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { bdd, type TestContext } from '../runner.ts';
import { runLlman, tempDir, lastRun } from './shared.ts';

function writePlugin(ctx: TestContext, name: string, body: string): string {
  let dir = ctx.fixtures['插件目录'] as string | undefined;
  if (!dir) {
    dir = tempDir('llman-bdd-plugins-');
    ctx.fixtures['插件目录'] = dir;
  }
  const path = join(dir, name);
  writeFileSync(path, body);
  chmodSync(path, 0o755);
  return path;
}

const ECHO_PLUGIN_BODY = '#!/bin/sh\nprintf \'args=%s\\n\' "$*"\nprintf \'config=%s\\n\' "$LLMAN_CONFIG_DIR"\n';

bdd.given('PATH 前置目录含可执行假插件 {name}', (ctx, params) => {
  writePlugin(ctx, params.name, ECHO_PLUGIN_BODY);
});

bdd.given('PATH 前置目录含以退出码 {code:d} 结束的假插件 {name}', (ctx, params) => {
  writePlugin(ctx, params.name, `#!/bin/sh\nexit ${params.code}\n`);
});

bdd.given('PATH 前置目录仅含 llman-real-plugin 而无 llman-no-such-cmd', (ctx) => {
  writePlugin(ctx, 'llman-real-plugin', ECHO_PLUGIN_BODY);
});

bdd.when('用 -C 临时配置目录运行 llman fake-echo', (ctx) => {
  const configDir = tempDir('llman-bdd-cfg-');
  ctx.fixtures['-C 配置目录'] = configDir;
  runLlman(ctx, ['-C', configDir, 'fake-echo']);
});

bdd.then('假插件进程 env 中 LLMAN_CONFIG_DIR 为 -C 临时配置目录', (ctx) => {
  const expected = ctx.fixtures['-C 配置目录'] as string;
  const stdout = lastRun(ctx).stdout;
  const actual = /^config=(.*)$/m.exec(stdout)?.[1]?.trim();
  if (actual !== expected) {
    throw new Error(`plugin LLMAN_CONFIG_DIR = "${actual}", expected "${expected}"\nstdout: ${stdout}`);
  }
});

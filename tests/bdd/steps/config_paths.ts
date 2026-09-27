// Steps for config-paths.feature: `llman --print-config-dir-path` resolves the
// config directory (pure read, no side effects) so priority / tilde / illegal
// path rules are assertable from stdout/stderr under controlled env.

import { bdd, type TestContext } from '../runner.ts';
import { lastRun, runLlman, tempDir } from './shared.ts';

bdd.when('不带任何配置来源运行 llman --print-config-dir-path', (ctx) => {
  // 不注入 LLMAN_CONFIG_DIR，HOME 指向受控临时目录 → 默认 ~/.config/llman。
  runLlman(ctx, ['--print-config-dir-path'], { omitConfigEnv: true });
});

bdd.when('同时以 -C 与环境变量运行 llman --print-config-dir-path', (ctx) => {
  const cliDir = tempDir('llman-bdd-cli-cfg-');
  const envDir = tempDir('llman-bdd-env-cfg-');
  ctx.fixtures['-C 目录'] = cliDir;
  ctx.fixtures['环境配置目录'] = envDir;
  runLlman(ctx, ['-C', cliDir, '--print-config-dir-path'], { extraEnv: { LLMAN_CONFIG_DIR: envDir } });
});

bdd.when('以环境变量 LLMAN_CONFIG_DIR 指向临时目录运行 llman --print-config-dir-path', (ctx) => {
  const envDir = tempDir('llman-bdd-env-cfg-');
  ctx.fixtures['环境配置目录'] = envDir;
  runLlman(ctx, ['--print-config-dir-path'], { extraEnv: { LLMAN_CONFIG_DIR: envDir } });
});

bdd.when('以环境变量 LLMAN_CONFIG_DIR 为空串运行 llman --print-config-dir-path', (ctx) => {
  runLlman(ctx, ['--print-config-dir-path'], { extraEnv: { LLMAN_CONFIG_DIR: '' } });
});

bdd.when('以 -C 空白路径运行 llman --print-config-dir-path', (ctx) => {
  runLlman(ctx, ['-C', '   ', '--print-config-dir-path']);
});

bdd.when('以环境变量 LLMAN_CONFIG_DIR 以 ~ 开头运行 llman --print-config-dir-path', (ctx) => {
  runLlman(ctx, ['--print-config-dir-path'], { extraEnv: { LLMAN_CONFIG_DIR: '~/xyz' } });
});

bdd.then('stdout 为 -C 指定目录', (ctx) => {
  const expected = ctx.fixtures['-C 目录'] as string;
  if (lastRun(ctx).stdout.trim() !== expected) {
    throw new Error(`stdout = "${lastRun(ctx).stdout.trim()}", expected -C dir "${expected}"`);
  }
});

bdd.then('stdout 为环境变量指定目录', (ctx) => {
  const expected = ctx.fixtures['环境配置目录'] as string;
  if (lastRun(ctx).stdout.trim() !== expected) {
    throw new Error(`stdout = "${lastRun(ctx).stdout.trim()}", expected env dir "${expected}"`);
  }
});

bdd.then('stdout 为 HOME 下的默认配置目录', (ctx) => {
  const expected = `${lastRun(ctx).home}/.config/llman`;
  if (lastRun(ctx).stdout.trim() !== expected) {
    throw new Error(`stdout = "${lastRun(ctx).stdout.trim()}", expected "${expected}"`);
  }
});

bdd.then('stdout 为 HOME 下的展开路径', (ctx) => {
  const expected = `${lastRun(ctx).home}/xyz`;
  if (lastRun(ctx).stdout.trim() !== expected) {
    throw new Error(`stdout = "${lastRun(ctx).stdout.trim()}", expected "${expected}"`);
  }
});

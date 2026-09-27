// Steps for the llman tool subcommands (clean-comments, rm-useless-dirs,
// sync-ignore, agents-md): file fixtures inside a scratch project that the
// generic `运行 llman {args:rest}` step executes against.

import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';

import { bdd, type TestContext } from '../runner.ts';
import { runLlman, tempDir } from './shared.ts';

function scratchProject(ctx: TestContext): string {
  let dir = ctx.fixtures['工作目录'] as string | undefined;
  if (!dir) {
    dir = tempDir('llman-bdd-tool-');
    ctx.fixtures['工作目录'] = dir;
    ctx.fixtures['sdd 项目'] = dir; // 相对路径断言以此为根
    spawnSync('git', ['init', '-q', '-b', 'main'], { cwd: dir });
  }
  return dir;
}

bdd.given('含注释 python 文件与空目录的临时项目', (ctx) => {
  const dir = scratchProject(ctx);
  writeFileSync(
    join(dir, 'code.py'),
    '# top comment\ndef f():\n    # inner comment\n    return 1\n',
  );
  mkdirSync(join(dir, 'empty-junk'), { recursive: true });
});

bdd.given('含 ignore 文件的 git 项目', (ctx) => {
  const dir = scratchProject(ctx);
  writeFileSync(join(dir, '.ignore'), 'node_modules/\n');
  writeFileSync(join(dir, '.cursorignore'), 'dist/\n');
});

bdd.given('含 agent init 文件的 git 项目', (ctx) => {
  const dir = scratchProject(ctx);
  writeFileSync(join(dir, 'AGENTS.md'), '# agents\n');
  mkdirSync(join(dir, '.claude'), { recursive: true });
  writeFileSync(join(dir, '.claude', 'settings.json'), '{}\n');
});

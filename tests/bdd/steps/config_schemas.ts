// Steps for config-schemas.feature r125: global config.yaml fixtures under a
// temp LLMAN_CONFIG_DIR (minimal valid shape: version + tools + skills.repo).

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { bdd, type TestContext } from '../runner.ts';
import { runLlman, tempDir, REPO_ROOT } from './shared.ts';

const CONFIG_HEADER = 'version: "0.1"\ntools: {}\n';

function writeGlobalConfig(ctx: TestContext, skillsSection: string): void {
  const configDir = tempDir('llman-bdd-cfg-');
  writeFileSync(join(configDir, 'config.yaml'), `${CONFIG_HEADER}${skillsSection}`);
  ctx.fixtures['配置目录'] = configDir;
}

// `self schema check` reads the committed artifacts/schema/ files, so this
// step runs from the repo root instead of the hermetic scratch cwd.
bdd.when('在非交互终端运行 llman self schema check', (ctx) => {
  runLlman(ctx, ['self', 'schema', 'check'], { cwd: REPO_ROOT });
});

bdd.given('全局 config.yaml 含 multi-repo skills 配置', (ctx) => {
  const repoPath = join(tempDir('llman-bdd-repo-'), 'repo');
  mkdirSync(repoPath, { recursive: true });
  writeGlobalConfig(ctx, `skills:\n  repo:\n  - name: demo\n    path: ${repoPath}\n`);
});

bdd.given('全局 config.yaml 含 legacy-dir skills 配置', (ctx) => {
  writeGlobalConfig(ctx, 'skills:\n  dir: /tmp/llman-bdd-legacy\n');
});

bdd.given('全局 config.yaml 含 missing-path skills 配置', (ctx) => {
  const missing = join(tempDir('llman-bdd-repo-'), 'nonexistent');
  writeGlobalConfig(ctx, `skills:\n  repo:\n  - name: missing\n    path: ${missing}\n`);
});

// ---------------------------------------------------------------------------
// r18/r49/r73: self schema generate / apply / check
// ---------------------------------------------------------------------------

import { bdd as bdd2, type TestContext as TC2 } from '../runner.ts';
import { runLlman as runL, lastRun as last } from './shared.ts';
import { projectRoot } from './sdd.ts';

bdd2.given('全局配置目录为空', (ctx) => {
  ctx.fixtures['配置目录'] = tempDir('llman-bdd-fresh-cfg-');
  ctx.fixtures['工作目录'] = tempDir('llman-bdd-work-');
});

bdd2.when('在项目子目录运行 llman self schema generate', (ctx) => {
  runL(ctx, ['self', 'schema', 'generate'], { cwd: join(projectRoot(ctx), 'llmanspec', 'changes') });
});

bdd2.when('在项目子目录运行 llman self schema apply', (ctx) => {
  runL(ctx, ['self', 'schema', 'apply'], { cwd: join(projectRoot(ctx), 'llmanspec', 'changes') });
});

bdd2.when('在临时工作目录运行 llman self schema generate', (ctx) => {
  runL(ctx, ['self', 'schema', 'generate']);
});

bdd2.when('运行 llman self schema check', (ctx) => {
  runL(ctx, ['self', 'schema', 'check']);
});

bdd2.then('全局 config.yaml 存在且含 yaml-language-server', (ctx) => {
  const configDir = ctx.fixtures['配置目录'] as string;
  const content = readFileSync(join(configDir, 'config.yaml'), 'utf8');
  if (!content.includes('yaml-language-server')) {
    throw new Error(`fresh global config.yaml lacks schema header\n${content.slice(0, 300)}`);
  }
});

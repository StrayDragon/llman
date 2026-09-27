// Steps for config-schemas.feature r125: global config.yaml fixtures under a
// temp LLMAN_CONFIG_DIR (minimal valid shape: version + tools + skills.repo).

import { mkdirSync, writeFileSync } from 'node:fs';
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

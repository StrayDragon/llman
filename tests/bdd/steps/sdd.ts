// Steps for the sdd-* capability specs: these scenarios exercise the REAL
// external llman-sdd binary reached through `llman sdd ...` delegation, inside
// a fixture project (git repo + `llman-sdd init` + committed llmanspec/).
//
// Hermeticity: the real llman-sdd binary is resolved once on the host
// (LLMAN_SDD_BIN override, then PATH + well-known global-bin dirs) and exposed
// to the child via a private bin dir on a controlled PATH — host plugins and
// the user's real config never leak in. The whole module degrades to skipped
// tests when no llman-sdd binary exists (CI installs @llman-sdd/cli).

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, symlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { bdd, type TestContext } from '../runner.ts';
import { lastRun, llmanBin, runLlman, tempDir, SYSTEM_PATH_DIRS } from './shared.ts';

// ---------------------------------------------------------------------------
// llman-sdd binary resolution + private bin dir
// ---------------------------------------------------------------------------

const SDD_EXTRA_DIRS = ['.bun/bin', '.local/bin', '.npm-global/bin', '.local/share/pnpm'];

export function findSddBin(): string | null {
  const override = process.env.LLMAN_SDD_BIN;
  if (override) return existsSync(override) ? override : null;
  const dirs = ['/usr/local/bin', '/usr/bin', ...(process.env.PATH ?? '').split(':').filter(Boolean), ...SDD_EXTRA_DIRS.map((d) => join(process.env.HOME ?? '', d))];
  for (const dir of dirs) {
    const candidate = join(dir, 'llman-sdd');
    if (existsSync(candidate)) return candidate;
  }
  return null;
}

function sddBinDir(ctx: TestContext): string {
  let dir = ctx.fixtures['sdd bin 目录'] as string | undefined;
  if (!dir) {
    const bin = findSddBin();
    if (!bin) throw new Error('llman-sdd binary not found (install: bun add -g @llman-sdd/cli)');
    dir = tempDir('llman-bdd-sddbin-');
    symlinkSync(bin, join(dir, 'llman-sdd'));
    ctx.fixtures['sdd bin 目录'] = dir;
  }
  return dir;
}

// ---------------------------------------------------------------------------
// Fixture project factory
// ---------------------------------------------------------------------------

function git(args: string[], cwd: string): void {
  const result = spawnSync('git', ['-c', 'user.email=bdd@local', '-c', 'user.name=bdd', ...args], { cwd, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed:\n${result.stderr}`);
}

function sddInit(project: string): void {
  const result = spawnSync(findSddBin()!, ['init', project], { cwd: project, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`llman-sdd init failed:\n${result.stderr}`);
}

const SAMPLE_FEATURE = (id: string, req: string) => `# language: zh-CN
# capability: ${id}
# purpose: fixture capability
# scope: llmanspec/

功能: ${id}

  @req:${req}
  规则: fixture 规则
    fixture 描述。

    场景: ${id}-ok
      假如 fixture 前置
      当 fixture 动作
      那么 fixture 断言
`;

function writeFeature(project: string, id: string, req: string, body?: string): void {
  writeFileSync(join(project, 'llmanspec', 'specs', `${id}.feature`), body ?? SAMPLE_FEATURE(id, req));
}

function commitAll(project: string, message: string): void {
  git(['add', '-A'], project);
  git(['commit', '-qm', message], project);
}

/** One sdd project per scenario: git(main) + `llman-sdd init` + sample capability + optional bdd section, committed clean. */
function sddProject(ctx: TestContext, bddMode: 'on' | 'off' | null = 'off'): string {
  let project = ctx.fixtures['sdd 项目'] as string | undefined;
  if (project) return project;
  project = tempDir('llman-bdd-proj-');
  ctx.fixtures['sdd 项目'] = project;
  ctx.fixtures['项目根'] = project;
  git(['init', '-q', '-b', 'main'], project);
  git(['config', 'user.email', 'bdd@local'], project);
  git(['config', 'user.name', 'bdd'], project);
  sddInit(project);
  writeFeature(project, 'sample', 'r1');
  if (bddMode === 'on') {
    writeFileSync(join(project, 'llmanspec', 'config.yaml'), 'schema: spec-driven\nlocale: en\nbdd:\n  run_command: "false"\n');
  }
  commitAll(project, 'fixture: init');
  return project;
}

function changeFixture(project: string, id: string, artifacts: 'full' | 'no-tasks' | 'proposal-only', needsSpecsChange?: boolean): void {
  const dir = join(project, 'llmanspec', 'changes', id);
  mkdirSync(dir, { recursive: true });
  const frontmatter = needsSpecsChange === undefined ? '---\ndepends_on: []\nblocks: []\n---\n' : `---\ndepends_on: []\nblocks: []\nneeds_specs_change: ${needsSpecsChange}\n---\n`;
  writeFileSync(
    join(dir, 'proposal.md'),
    `${frontmatter}# ${id}\n\n## Why\n\nTODO: Why is this change needed?\n\n## What Changes\n\nTODO: Bullet list of what changes.\n`,
  );
  if (artifacts !== 'proposal-only') writeFileSync(join(dir, 'design.md'), '# Design\n\nfixture design\n');
  if (artifacts === 'full') writeFileSync(join(dir, 'tasks.md'), '# Tasks\n\n- [x] t1\n');
}

/** Build an active change up to the requested attachment state (yes/skip run `change start` → Full + bound branch). */
function makeChange(ctx: TestContext, id: string, artifacts: 'full' | 'no-tasks' | 'proposal-only', attach: 'yes' | 'no' | 'skip'): void {
  const project = sddProject(ctx);
  changeFixture(project, id, artifacts, attach === 'skip' ? false : undefined);
  commitAll(project, `fixture: change ${id}`);
  if (attach === 'yes' || attach === 'skip') {
    runSdd(ctx, ['change', 'start', id]);
    if (lastRun(ctx).code !== 0) throw new Error(`fixture change start ${id} failed:\n${lastRun(ctx).stderr}`);
    // change start 把 binding 写进 frontmatter（留在工作区）——fixture 提交它，
    // 使 clean-tree 门在后续断言中为 true。
    commitAll(project, `fixture: binding ${id}`);
  }
}

// ---------------------------------------------------------------------------
// Execution: `llman sdd ...` through the real delegated binary
// ---------------------------------------------------------------------------

export function splitArgs(s: string): string[] {
  const out: string[] = [];
  let cur = '';
  let quote: string | null = null;
  for (const ch of s) {
    if (quote) {
      if (ch === quote) quote = null;
      else cur += ch;
    } else if (ch === '"' || ch === "'") quote = ch;
    else if (/\s/.test(ch)) {
      if (cur) {
        out.push(cur);
        cur = '';
      }
    } else cur += ch;
  }
  if (cur) out.push(cur);
  return out;
}

export function runSdd(ctx: TestContext, args: string[]): void {
  const project = ctx.fixtures['sdd 项目'] as string | undefined;
  if (!project) throw new Error('no fixture sdd project — add a 已初始化… Given first');
  const result = spawnSync(llmanBin(), ['sdd', ...args], {
    cwd: project,
    env: {
      PATH: [ctx.fixtures['sdd bin 目录'] ?? sddBinDir(ctx), ctx.fixtures['插件目录'], SYSTEM_PATH_DIRS].filter(Boolean).join(':'),
      HOME: tempDir(),
      LLMAN_CONFIG_DIR: tempDir(),
    },
    encoding: 'utf8',
  });
  ctx.fixtures['上次运行'] = {
    code: result.status ?? 1,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
  };
}

bdd.when('运行 llman sdd {args:rest}', (ctx, params) => {
  const args = splitArgs(params.args);
  // Inside a fixture project the real delegated binary runs; without one
  // (e.g. errors-exit delegation-failure paths) fall back to the hermetic
  // llman-only run where llman-sdd is intentionally absent.
  if (ctx.fixtures['sdd 项目']) runSdd(ctx, args);
  else runLlman(ctx, ['sdd', ...args]);
});
bdd.when('在非交互终端运行 llman sdd {args:rest}', (ctx, params) => {
  const args = splitArgs(params.args);
  if (ctx.fixtures['sdd 项目']) runSdd(ctx, args);
  else runLlman(ctx, ['sdd', ...args]);
});

// ---------------------------------------------------------------------------
// Givens: fixture projects and shapes
// ---------------------------------------------------------------------------

bdd.given('已初始化 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  // 标准活动 change：attach/start/diff 类场景的公共前置。
  changeFixture(project, 'add-scen', 'proposal-only');
  commitAll(project, 'fixture: standard add-scen change');
});

bdd.given('已初始化含跨 spec 重复 req_id 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  writeFeature(project, 'foo', 'r99');
  writeFeature(project, 'bar', 'r99');
  commitAll(project, 'fixture: duplicate req ids');
});

bdd.given('已初始化含跨 capability 重复 req_id 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  writeFeature(project, 'foo', 'r99');
  writeFeature(project, 'bar', 'r99');
  commitAll(project, 'fixture: duplicate req ids');
});

bdd.given('已初始化含多个 capability 且无占位符计数 run_command 的 sdd 项目', (ctx) => {
  const project = sddProject(ctx, 'off');
  writeFeature(project, 'sample2', 'r2');
  writeFileSync(
    join(project, 'llmanspec', 'config.yaml'),
    'schema: spec-driven\nlocale: en\nbdd:\n  run_command: "sh -c \'echo x >> .bdd-run-count\'"\n',
  );
  commitAll(project, 'fixture: batch dedup harness');
});

bdd.given('已初始化含损坏 proposal 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  const dir = join(project, 'llmanspec', 'changes', 'broken');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'proposal.md'), '---\ndepends_on: []\nblocks: []\n---\n# broken\n\n## Why\n\nw\n');
  writeFileSync(join(dir, 'tasks.md'), '# Tasks\n\n- [ ] t1\n');
  commitAll(project, 'fixture: broken proposal (tasks without design)');
});

bdd.given('已初始化含已占用全局 req_id 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  // 工厂 sample 已占用 r1——注册表口径下的已占用形态。
  sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
});

bdd.given('项目中存在技能目录 {name}', (ctx, params) => {
  const project = sddProject(ctx);
  const dir = join(project, '.agents', 'skills', params.name);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'SKILL.md'), '---\nname: "x"\ndescription: "x"\n---\nx\n');
});

bdd.given('项目 extra_skills 包含 {name}', (ctx, params) => {
  const project = sddProject(ctx);
  const configPath = join(project, 'llmanspec', 'config.yaml');
  const config = readFileSync(configPath, 'utf8');
  writeFileSync(configPath, config.replace('locale: en', () => `locale: en\nextra_skills:\n  - ${params.name}`));
  commitAll(project, 'fixture: extra_skills');
});

bdd.given('已初始化含 change_id pattern 与 archive 形态存量的 sdd 项目且存在违规 active change "{id:rest}"', (ctx, params) => {
  const project = sddProject(ctx, 'off');
  const configPath = join(project, 'llmanspec', 'config.yaml');
  const config = readFileSync(configPath, 'utf8');
  writeFileSync(configPath, config.replace('locale: en', () => "locale: en\nchange_id:\n  pattern: '^[a-z0-9][a-z0-9-]*$'"));
  changeFixture(project, params.id.replace(/"/g, ''), 'proposal-only');
  const archived = join(project, 'llmanspec', 'changes', 'archive', '2026-09-13-c20-legacy');
  mkdirSync(archived, { recursive: true });
  writeFileSync(join(archived, 'proposal.md'), '---\ndepends_on: []\nblocks: []\n---\n# c20-legacy\n');
  commitAll(project, 'fixture: change_id pattern');
});

bdd.given('已初始化含 change_id template 的 sdd 项目', (ctx) => {
  const project = sddProject(ctx, 'off');
  const configPath = join(project, 'llmanspec', 'config.yaml');
  const config = readFileSync(configPath, 'utf8');
  writeFileSync(configPath, config.replace('locale: en', () => "locale: en\nchange_id:\n  template: 'c{{ llman_sdd_unique_id }}-{{ verb }}-{{ subject }}'"));
  commitAll(project, 'fixture: change_id template');
});

bdd.given('已初始化含 change_id 段且 delayed-changes 深层目录含更大号的 sdd 项目', (ctx) => {
  const project = sddProject(ctx, 'off');
  const configPath = join(project, 'llmanspec', 'config.yaml');
  const config = readFileSync(configPath, 'utf8');
  writeFileSync(configPath, config.replace('locale: en', () => "locale: en\nchange_id:\n  template: '{{ verb }}-{{ subject }}'"));
  const deep = join(project, 'llmanspec', 'delayed-changes', 'deep', 'c2619-legacy');
  mkdirSync(deep, { recursive: true });
  writeFileSync(join(deep, 'proposal.md'), '---\ndepends_on: []\nblocks: []\n---\n# c2619-legacy\n');
  commitAll(project, 'fixture: delayed deeper ids');
});

bdd.given('已初始化含失效 scope 路径 spec 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  writeFeature(project, 'sample', 'r1', SAMPLE_FEATURE('sample', 'r1').replace('# scope: llmanspec/', '# scope: docs/gone'));
  commitAll(project, 'fixture: stale scope');
});

bdd.given('已初始化含遗留 spec.toon 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  const dir = join(project, 'llmanspec', 'specs', 'legacy');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'spec.toon'), 'legacy toon content\n');
  commitAll(project, 'fixture: legacy spec.toon');
});

bdd.given('已初始化含遗留 spec.toon 的 legacy capability 且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  const dir = join(project, 'llmanspec', 'specs', 'legacy');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'spec.toon'), 'legacy toon content\n');
  commitAll(project, 'fixture: legacy spec.toon capability');
});

bdd.given('已初始化含扁平 capability 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  writeFeature(project, 'flatcap', 'r20');
  commitAll(project, 'fixture: flat capability');
});

bdd.given('已初始化含同 id 扁平与目录冲突的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  writeFeature(project, 'foo', 'r21');
  const dir = join(project, 'llmanspec', 'specs', 'foo');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'foo.feature'), SAMPLE_FEATURE('foo', 'r22'));
  commitAll(project, 'fixture: flat/dir collision');
});

bdd.given('已初始化含多 .feature 目录 capability 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  const dir = join(project, 'llmanspec', 'specs', 'multi');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'multi.feature'), SAMPLE_FEATURE('multi', 'r23'));
  writeFileSync(join(dir, 'other.feature'), SAMPLE_FEATURE('multi-other', 'r24'));
  commitAll(project, 'fixture: multi-feature dir');
});

bdd.given('已初始化含异名单文件目录 capability 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  const dir = join(project, 'llmanspec', 'specs', 'solocap');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'other.feature'), SAMPLE_FEATURE('solocap', 'r25'));
  commitAll(project, 'fixture: misnamed solo dir');
});

bdd.given('已初始化含单文件目录 capability 的 sdd 项目且 bdd 配置为 "{mode}"', (ctx, params) => {
  const project = sddProject(ctx, params.mode.replace(/"/g, '') === 'on' ? 'on' : 'off');
  const dir = join(project, 'llmanspec', 'specs', 'scoped');
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, 'scoped.feature'), SAMPLE_FEATURE('scoped', 'r26'));
  commitAll(project, 'fixture: single-file dir');
});

// ---------------------------------------------------------------------------
// Givens: change lifecycle fixtures
// ---------------------------------------------------------------------------

bdd.given('变更 {id} 含 proposal design tasks 且 attach 状态为 "{state}"', (ctx, params) => {
  const attach = params.state.replace(/"/g, '') as 'yes' | 'no' | 'skip';
  makeChange(ctx, params.id, 'full', attach);
});

bdd.given('变更 {id} 仅含 proposal', (ctx, params) => {
  makeChange(ctx, params.id, 'proposal-only', 'no');
});

bdd.given('变更 {id} 含 proposal 与 design 不含 tasks', (ctx, params) => {
  makeChange(ctx, params.id, 'no-tasks', 'no');
});

bdd.given('变更 {id} 绑定于已提交 specs 改动的分支', (ctx, params) => {
  makeChange(ctx, params.id, 'full', 'yes');
  const project = sddProject(ctx);
  writeFileSync(join(project, 'llmanspec', 'specs', 'sample.feature'), SAMPLE_FEATURE('sample', 'r1').replace('fixture 描述。', 'fixture 描述（分支上已编辑）。'));
  commitAll(project, 'fixture: spec edit on branch');
});

bdd.given('变更 {id} 绑定于先前规则编辑已合入默认分支零推送的历史', (ctx, params) => {
  const project = sddProject(ctx, 'off');
  writeFileSync(join(project, 'llmanspec', 'specs', 'sample.feature'), SAMPLE_FEATURE('sample', 'r1').replace('fixture 描述。', 'fixture 描述（规则编辑已合入默认分支）。'));
  commitAll(project, 'fixture: rule edit merged to default');
  // 先前规则编辑已在默认分支历史中——本 change 无新增 live 编辑，specs-landed 门走 needs_specs_change=false 豁免。
  makeChange(ctx, params.id, 'full', 'skip');
});

bdd.given('存在可收尾的完整 change {id}', (ctx, params) => {
  makeChange(ctx, params.id, 'full', 'yes');
});

// ---------------------------------------------------------------------------
// cli.feature r112: change-id prefix matching (delegation + real tool)
// ---------------------------------------------------------------------------

bdd.given('存在 active change 和 archived change 且含 c123-fix-bug', (ctx) => {
  const project = sddProject(ctx, 'off');
  const proposal = '---\ndepends_on: []\nblocks: []\n---\n# c123-fix-bug\n\n## Why\n\nwhy\n\n## What Changes\n\nwhat\n';
  const active = join(project, 'llmanspec', 'changes', 'c123-fix-bug');
  mkdirSync(active, { recursive: true });
  writeFileSync(join(active, 'proposal.md'), proposal);
  const archived = join(project, 'llmanspec', 'changes', 'archive', '2026-01-01-c123-fix-bak');
  mkdirSync(archived, { recursive: true });
  writeFileSync(join(archived, 'proposal.md'), '---\ndepends_on: []\nblocks: []\n---\n# c123-fix-bak\n');
  commitAll(project, 'fixture: c123-fix-bug active + archived');
});

bdd.when('用前缀运行 llman sdd show c12', (ctx) => {
  runSdd(ctx, ['show', 'c12']);
});

bdd.when('用前缀 c123 运行 llman sdd show c123', (ctx) => {
  runSdd(ctx, ['show', 'c123']);
});

bdd.when('用前缀 c123 运行 llman sdd show c123 --output json', (ctx) => {
  runSdd(ctx, ['show', 'c123', '--output', 'json']);
});

bdd.then('对应的完整 change 被找到且输出正确', (ctx) => {
  if (!lastRun(ctx).stdout.includes('c123-fix-bug')) {
    throw new Error(`stdout does not contain "c123-fix-bug"\nstdout: ${lastRun(ctx).stdout}`);
  }
});

// ---------------------------------------------------------------------------
// Assertions: JSON keys, relative paths, git state (fixture-project rooted)
// ---------------------------------------------------------------------------

function jsonOut(ctx: TestContext): any {
  try {
    return JSON.parse(lastRun(ctx).stdout);
  } catch {
    throw new Error(`stdout is not valid JSON:\n${lastRun(ctx).stdout}`);
  }
}

function jsonKey(obj: any, key: string): any {
  let cur = obj;
  for (const part of key.split('.')) {
    if (cur == null || !(part in Object(cur))) throw new Error(`stdout JSON has no key "${key}"`);
    cur = cur[part];
  }
  return cur;
}

export function projectRoot(ctx: TestContext): string {
  const project = ctx.fixtures['sdd 项目'] as string | undefined;
  if (!project) throw new Error('no fixture sdd project for a 相对路径 step');
  return project;
}

bdd.then('stdout 为合法 JSON', (ctx) => {
  jsonOut(ctx);
});

bdd.then('stdout 为合法 JSON 且含 JSON 键 {key}', (ctx, params) => {
  jsonKey(jsonOut(ctx), params.key);
});

bdd.then('退出码为零且 stdout 为合法 JSON 且含 JSON 键 reqId', (ctx) => {
  if (lastRun(ctx).code !== 0) throw new Error(`expected exit 0, got ${lastRun(ctx).code}\nstderr: ${lastRun(ctx).stderr}`);
  jsonKey(jsonOut(ctx), 'reqId');
});

bdd.then('退出码为零且 stdout 为合法 JSON 且含 JSON 键 reqId 且含 JSON 键 capability', (ctx) => {
  if (lastRun(ctx).code !== 0) throw new Error(`expected exit 0, got ${lastRun(ctx).code}\nstderr: ${lastRun(ctx).stderr}`);
  const json = jsonOut(ctx);
  jsonKey(json, 'reqId');
  jsonKey(json, 'capability');
});

bdd.then('stdout 的 JSON 键 {key} 为 "{value}"', (ctx, params) => {
  const json = jsonOut(ctx);
  const actual = jsonKey(json, params.key);
  if (String(actual) !== params.value) {
    throw new Error(`JSON key "${params.key}" = ${JSON.stringify(actual)}, expected "${params.value}"\njson: ${JSON.stringify(json).slice(0, 900)}`);
  }
});

bdd.then('stdout 的 JSON 键 {key} 为 true', (ctx, params) => {
  const actual = jsonKey(jsonOut(ctx), params.key);
  if (actual !== true) throw new Error(`JSON key "${params.key}" = ${JSON.stringify(actual)}, expected true`);
});

bdd.then('stdout 的 JSON 键 {key} 为 false', (ctx, params) => {
  const actual = jsonKey(jsonOut(ctx), params.key);
  if (actual !== false) throw new Error(`JSON key "${params.key}" = ${JSON.stringify(actual)}, expected false`);
});

bdd.then('stdout 的 JSON 键 {key} 为数字', (ctx, params) => {
  const actual = jsonKey(jsonOut(ctx), params.key);
  if (typeof actual !== 'number') throw new Error(`JSON key "${params.key}" is not a number: ${JSON.stringify(actual)}`);
});

bdd.then('stdout 的 JSON 键 signals 含 kind 为 {kind} 的条目且 count 为 {n:d}', (ctx, params) => {
  const signals = jsonKey(jsonOut(ctx), 'signals');
  const entry = (signals as any[]).find((s) => s.kind === params.kind);
  if (!entry) throw new Error(`signals has no kind "${params.kind}"\n${JSON.stringify(signals)}`);
  if (entry.count !== Number(params.n)) throw new Error(`signals kind "${params.kind}" count = ${entry.count}, expected ${params.n}`);
});

bdd.then('相对路径 {path} 存在', (ctx, params) => {
  if (!existsSync(join(projectRoot(ctx), params.path))) throw new Error(`relative path missing: ${params.path}`);
});

bdd.then('相对路径 {path} 不存在', (ctx, params) => {
  if (existsSync(join(projectRoot(ctx), params.path))) throw new Error(`relative path unexpectedly exists: ${params.path}`);
});

bdd.then('相对路径 {path} 内容包含 {text:rest}', (ctx, params) => {
  const content = readFileSync(join(projectRoot(ctx), params.path), 'utf8');
  if (!content.includes(params.text)) throw new Error(`${params.path} does not contain "${params.text}"\n${content}`);
});

bdd.then('相对路径 "{path}" 行数为 {n:d}', (ctx, params) => {
  const content = readFileSync(join(projectRoot(ctx), params.path), 'utf8');
  const lines = content.split('\n').filter((l) => l.length > 0).length;
  if (lines !== Number(params.n)) throw new Error(`${params.path} has ${lines} lines, expected ${params.n}`);
});

bdd.then('退出码非零且 stderr 包含 {text:rest}', (ctx, params) => {
  if (lastRun(ctx).code === 0) throw new Error('expected non-zero exit, got 0');
  if (!lastRun(ctx).stderr.includes(params.text)) throw new Error(`stderr does not contain "${params.text}"\nstderr: ${lastRun(ctx).stderr}`);
});

bdd.then('最近提交说明包含 {text:rest}', (ctx, params) => {
  const result = spawnSync('git', ['log', '-1', '--format=%s'], { cwd: projectRoot(ctx), encoding: 'utf8' });
  if (!(result.stdout ?? '').includes(params.text)) throw new Error(`latest commit subject does not contain "${params.text}"\n${result.stdout}`);
});

bdd.then('最近提交说明不含 {text:rest}', (ctx, params) => {
  const result = spawnSync('git', ['log', '-1', '--format=%s'], { cwd: projectRoot(ctx), encoding: 'utf8' });
  if ((result.stdout ?? '').includes(params.text)) throw new Error(`latest commit subject unexpectedly contains "${params.text}"\n${result.stdout}`);
});

bdd.then('工作区存在未提交改动', (ctx) => {
  const result = spawnSync('git', ['status', '--porcelain'], { cwd: projectRoot(ctx), encoding: 'utf8' });
  if (!(result.stdout ?? '').trim()) throw new Error('expected uncommitted changes, working tree is clean');
});

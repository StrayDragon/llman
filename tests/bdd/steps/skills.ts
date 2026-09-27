// Steps for skill-rendering contracts (sdd-structured-skill-prompts and the
// skill-governance rules in sdd-workflow): enable optional skills via
// extra_skills, resync with init --update, then assert the rendered
// SKILL.md artifacts inside the fixture project.

import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { bdd, type TestContext } from '../runner.ts';
import { runSdd, sddProject } from './sdd.ts';
import { lastRun } from './shared.ts';

bdd.given('已启用可选技能 {name} 的 sdd 项目', (ctx, params) => {
  const project = sddProject(ctx, 'off');
  const configPath = join(project, 'llmanspec', 'config.yaml');
  const config = readFileSync(configPath, 'utf8');
  // 函数形式 replace：避免 $ 系列特殊替换模式。
  const updated = config.replace('locale: zh-Hans', () => `locale: zh-Hans\nextra_skills:\n  - ${params.name}`);
  writeFileSync(configPath, updated);
  runSdd(ctx, ['init', '--update']);
  if (lastRun(ctx).code !== 0) throw new Error(`init --update failed:\n${lastRun(ctx).stderr}`);
});

bdd.then('渲染技能 {name} 存在', (ctx, params) => {
  const project = ctx.fixtures['sdd 项目'] as string;
  if (!existsSync(join(project, '.agents', 'skills', params.name, 'SKILL.md'))) {
    throw new Error(`rendered skill missing: ${params.name}`);
  }
});

bdd.then('渲染技能 {name} 不存在', (ctx, params) => {
  const project = ctx.fixtures['sdd 项目'] as string;
  if (existsSync(join(project, '.agents', 'skills', params.name, 'SKILL.md'))) {
    throw new Error(`rendered skill unexpectedly exists: ${params.name}`);
  }
});

bdd.then('渲染技能 {name} 内容包含 {text:rest}', (ctx, params) => {
  const project = ctx.fixtures['sdd 项目'] as string;
  const path = join(project, '.agents', 'skills', params.name, 'SKILL.md');
  const content = readFileSync(path, 'utf8');
  if (!content.includes(params.text)) {
    throw new Error(`rendered skill ${params.name} does not contain "${params.text}"`);
  }
});

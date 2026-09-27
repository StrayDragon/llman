// BDD entry point — executes the llmanspec/specs nested scenarios that have
// step bindings registered (importing a steps module registers its steps as
// a side effect), then turns each bound feature into a bun:test describe.
//
// The allowlist declares the bound set explicitly: a feature not listed here
// is not executed at all. Features marked `skipIf` degrade to skipped tests
// when the real llman-sdd binary is absent (CI installs @llman-sdd/cli so
// they run there; bare-rule-only features simply produce zero scenarios).
// Remaining unbound surface (97 bare rules without executable acceptance) is
// tracked as review `pending` metrics, not hidden here.

import { join } from 'node:path';

import './steps/shared.ts';
import './steps/cli.ts';
import './steps/config_schemas.ts';
import './steps/config_paths.ts';
import './steps/repo.ts';
import './steps/skills.ts';
import './steps/tools.ts';
import './steps/sdd.ts';
import { runFeature, type TestContext } from './runner.ts';
import { findSddBin } from './steps/sdd.ts';

const SPECS_DIR = join(import.meta.dir, '..', '..', 'llmanspec', 'specs');
const sddMissing = () => findSddBin() === null;

interface BoundFeature {
  file: string;
  /** Scenario names to execute; omit to run every scenario in the feature. */
  include?: RegExp;
  /** Skip the whole entry when true at load time (e.g. llman-sdd absent). */
  skipIf?: () => boolean;
}

const BOUND_FEATURES: BoundFeature[] = [
  { file: 'cli.feature', include: /^external-/ },
  { file: 'cli.feature', include: /^prefix-match/, skipIf: sddMissing },
  { file: 'errors-exit.feature' },
  { file: 'config-schemas.feature' },
  { file: 'config-paths.feature' },
  { file: 'sdd-bdd-mode-compat.feature', skipIf: sddMissing },
  { file: 'sdd-context.feature', skipIf: sddMissing },
  { file: 'sdd-review.feature', skipIf: sddMissing },
  { file: 'sdd-workflow.feature', skipIf: sddMissing },
  { file: 'spec-format.feature', skipIf: sddMissing },
  { file: 'sdd-structured-skill-prompts.feature', skipIf: sddMissing },
];

function makeContext(): TestContext {
  return { fixtures: {} };
}

for (const bound of BOUND_FEATURES) {
  runFeature(join(SPECS_DIR, bound.file), makeContext, bound.include, bound.skipIf);
}

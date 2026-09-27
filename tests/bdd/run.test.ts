// BDD entry point — executes the llmanspec/specs nested scenarios that have
// step bindings registered (importing a steps module registers its steps as
// a side effect), then turns each bound feature into a bun:test describe.
//
// The allowlist grows as step definitions land: a feature not listed here is
// not executed at all (unbound scenarios must fail nowhere; they surface as
// review `pending` metrics instead). External-behavior scenarios (sdd-*)
// and fixtures needing the real llman-sdd binary are deliberately later
// phases — e.g. cli.feature r112 (change-id prefix match) needs a fixture
// llmanspec tree and is deferred with them.

import { join } from 'node:path';

import './steps/shared.ts';
import './steps/cli.ts';
import './steps/config_schemas.ts';
import { runFeature, type TestContext } from './runner.ts';

const SPECS_DIR = join(import.meta.dir, '..', '..', 'llmanspec', 'specs');

interface BoundFeature {
  file: string;
  /** Scenario names to execute; omit to run every scenario in the feature. */
  include?: RegExp;
}

const BOUND_FEATURES: BoundFeature[] = [
  { file: 'cli.feature', include: /^external-/ },
  { file: 'errors-exit.feature' },
  { file: 'config-schemas.feature' },
];

function makeContext(): TestContext {
  return { fixtures: {} };
}

for (const bound of BOUND_FEATURES) {
  runFeature(join(SPECS_DIR, bound.file), makeContext, bound.include);
}

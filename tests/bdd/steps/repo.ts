// Steps for repository-self-inspection scenarios (tests-ci, nightly
// toolchain governance, migrations SOP): assertions run against THIS
// repository's real working tree.

import { bdd, type TestContext } from '../runner.ts';
import { REPO_ROOT } from './shared.ts';

bdd.given('本仓库真实工作区', (ctx) => {
  // 相对路径断言步骤以 'sdd 项目' 为根——指向本仓库即得仓库自检语义。
  ctx.fixtures['sdd 项目'] = REPO_ROOT;
  ctx.fixtures['项目根'] = REPO_ROOT;
});

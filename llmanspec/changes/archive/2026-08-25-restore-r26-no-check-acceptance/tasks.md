# Tasks: restore-r26-no-check-acceptance

## 测试边界(seam)

- seam:`llman sdd validate <spec> --strict --no-check` CLI 子进程(既有 step 库直接驱动,不新增 step)
- 断言面:退出码为零 + stderr 不含 `BDD check failed`(与 v0.0.66 原场景同口径)

## 垂直切片

### t1: Specs landing——恢复 r26 验收场景
- [x] 绑定分支上在 `sdd-bdd-mode-compat.feature` 现有 `@req:r26` executable 场景后,追加「BDD-on 时 validate --no-check 跳过 runner」(`@executable` + `@req:r26`,假如/当/那么三步)并 commit

### t2: 验收全绿
- [x] `llman sdd validate restore-r26-no-check-acceptance --strict` 通过
- [x] `cargo +nightly test --features bdd` 49 场景全绿(48 + 恢复的 1)

# language: zh-CN
# managed by llman sdd partition-migrate
功能: dependency-upgrade-workflow

  @req:r1
  场景: 升级开始时先更新 lockfile 并跑校验
    假如 维护者开始为本仓库执行依赖升级
    当 维护者先更新 {lockfile} 并运行校验
    那么 在改动 {manifest} 中的依赖版本约束前先完成 lockfile 更新

  @req:r1
  场景: lockfile 更新因版本约束不足时才最小化改动 manifest
    假如 某次升级需要 manifest 约束改动才能编译或通过校验
    当 维护者应用约束改动
    那么 仅应用所需的边界更新
    而且 以基于 nightly 的校验验证结果

  @req:r1
  场景: manifest 约束改动最小且经质量门验证
    假如 {manifest} 中的依赖版本约束发生改动
    当 维护者完成改动
    那么 改动最小且限于兼容性需要
    而且 经项目质量门验证

  @req:r1
  场景: 升级批次完成后可验证并保留回退路径
    假如 维护者完成一批依赖升级
    当 维护者准备合并
    那么 可展示基于 nightly 的校验已通过
    而且 回退到上一 lock 状态仍可行

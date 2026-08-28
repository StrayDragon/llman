#[macro_use]
extern crate rust_i18n;

i18n!("locales");

use std::sync::OnceLock;

pub mod arg_utils;
pub mod cli;
pub mod config;
pub mod config_schema;
pub mod editor;
pub mod error;
// 顶层工具层已拆至 llman-core crate（T11）；此处重导出保持历史路径零漂移：
// `crate::fs_utils` / `llman::path_utils` / `llman_core::git_utils` 均可。
pub(crate) use llman_core::fs_utils;
pub use llman_core::{env_safety, git_utils, managed_block, path_utils, schema_utils};
pub mod prompts;
// sdd 模块树已拆至 llman-sdd crate（T12）；crate 内保留 `pub mod sdd` 保内部
// 路径，门面重导出该模块 → `crate::sdd::…` 与 `llman::sdd::…` 零漂移。
pub use llman_sdd::sdd;
pub mod self_command;
pub mod skills;
pub mod tool;
pub mod x;

#[cfg(test)]
pub mod test_utils;

static LOCALE_INIT: OnceLock<()> = OnceLock::new();

pub fn init_locale() {
    LOCALE_INIT.get_or_init(|| {
        rust_i18n::set_locale("en");
    });
}

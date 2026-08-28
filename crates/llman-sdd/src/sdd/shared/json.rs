//! JSON output helpers shared by sdd subcommands (`list` / `show` / `validate`).

use anyhow::Result;

/// Print a JSON value, pretty by default or compact with `--compact-json`.
pub(crate) fn print_json(value: &serde_json::Value, compact: bool) -> Result<()> {
    if compact {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}

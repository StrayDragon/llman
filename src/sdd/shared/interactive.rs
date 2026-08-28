use rust_i18n::t;
use std::io::IsTerminal;

pub(crate) fn is_interactive(no_interactive: bool) -> bool {
    if no_interactive {
        return false;
    }
    std::io::stdin().is_terminal()
}

/// Assemble the "nothing to X" hint shown when stdin is not a terminal:
/// a headline line, command attempt lines, and the shared tail line telling
/// the user to re-run in an interactive terminal.
pub(crate) fn non_interactive_hint_message(headline: String, command_hints: &[String]) -> String {
    let mut lines = Vec::with_capacity(command_hints.len() + 2);
    lines.push(headline);
    lines.extend(command_hints.iter().cloned());
    lines.push(t!("sdd.shared.non_interactive.tail").to_string());
    lines.join("\n")
}

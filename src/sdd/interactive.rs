use std::io::IsTerminal;

pub fn is_interactive(no_interactive: bool) -> bool {
    if no_interactive {
        return false;
    }
    std::io::stdin().is_terminal()
}

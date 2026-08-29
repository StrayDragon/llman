//! Workspace-wide inquire theme (modern terminal look, consistent symbols).
//!
//! Applied once at startup via [`init`] — every inquire prompt in the CLI
//! (Select / MultiSelect / Confirm / Text / …) inherits it through
//! `inquire::set_global_render_config`.
//!
//! Symbol vocabulary (kept deliberately consistent):
//!
//! | element            | symbol | meaning                       |
//! |--------------------|--------|-------------------------------|
//! | prompt             | `❯`    | asking (cyan, bold)           |
//! | answered           | `✔`    | done (green)                  |
//! | highlighted option | `▸`    | cursor (cyan, bold)           |
//! | selected checkbox  | `◉`    | on (cyan)                     |
//! | unselected checkbox| `○`    | off (grey)                    |
//! | scroll hints       | `⌃⌄`   | more items above/below        |
//! | cancelled          | `✗`    | prompt cancelled (dim)        |

use inquire::ui::{Attributes, Color, IndexPrefix, RenderConfig, StyleSheet, Styled};

/// Install the global render config. Call once, before any prompt runs.
pub fn init() {
    inquire::set_global_render_config(theme());
}

fn theme() -> RenderConfig<'static> {
    RenderConfig::default_colored()
        .with_prompt_prefix(
            Styled::new("❯")
                .with_fg(Color::LightCyan)
                .with_attr(Attributes::BOLD),
        )
        .with_answered_prompt_prefix(Styled::new("✔").with_fg(Color::LightGreen))
        .with_highlighted_option_prefix(
            Styled::new("▸")
                .with_fg(Color::LightCyan)
                .with_attr(Attributes::BOLD),
        )
        .with_selected_checkbox(Styled::new("◉").with_fg(Color::LightCyan))
        .with_unselected_checkbox(Styled::new("○").with_fg(Color::DarkGrey))
        .with_scroll_up_prefix(Styled::new("⌃").with_fg(Color::DarkGrey))
        .with_scroll_down_prefix(Styled::new("⌄").with_fg(Color::DarkGrey))
        .with_option_index_prefix(IndexPrefix::None)
        .with_canceled_prompt_indicator(Styled::new("✗ cancelled").with_fg(Color::DarkGrey))
        .with_help_message(StyleSheet::empty().with_fg(Color::DarkGrey))
        .with_answer(StyleSheet::empty().with_fg(Color::LightCyan))
}

//! inquire-based multi-select picker for skills.
//!
//! Replaces the former ratatui fullscreen tree picker with a two-phase,
//! ergonomics-first flow:
//!
//! 1. **Preset groups** (only when presets exist; optional) — checking a
//!    group preselects all of its skills in phase 2. Unchecking a group here
//!    means "don't add wholesale"; it never silently removes already-selected
//!    skills. Pressing Enter with nothing checked skips group selection.
//! 2. **Skills** — the final editor: every skill is visible, pre-checked with
//!    the union of chosen group members and the previously selected set, and
//!    can be toggled individually.
//!
//! Grouping survives as stable option ordering (preset-grouped rows first,
//! then an "other" group for uncovered skills); filtering, cursor handling
//! and terminal lifecycle are delegated to inquire's `MultiSelect`.

use anyhow::Result;
use inquire::MultiSelect;
use inquire::error::InquireError;
use inquire::list_option::ListOption;
use inquire::type_aliases::Scorer;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

/// The interactive filter matches option text; keep the page generous.
const PAGE_SIZE: usize = 15;

const PRESET_HELP_MESSAGE: &str = "↑↓/jk move · space toggle · type to filter · \
Enter to continue (empty = skip groups) · Esc cancel";
const SKILL_HELP_MESSAGE: &str =
    "↑↓/jk move · space toggle · type to filter · Enter confirm · Esc cancel";

#[derive(Clone, Debug)]
pub enum PickerEntryKind {
    Preset { skill_ids: Vec<String> },
    Skill { skill_id: String },
}

#[derive(Clone, Debug)]
pub struct PickerEntry {
    pub label: String,
    pub kind: PickerEntryKind,
}

/// One selectable row: a concrete skill and the text shown for it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PickerOption {
    skill_id: String,
    label: String,
}

impl Display for PickerOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// A preset group offered in the optional first phase.
#[derive(Clone, Debug)]
struct PresetRow {
    label: String,
    skill_ids: Vec<String>,
}

/// Prompt the user to select a set of skills (two-phase; see module docs).
///
/// Returns `Ok(None)` when the user cancels (Esc / Ctrl+C), otherwise the
/// selected skill ids (possibly empty).
pub fn pick(
    prompt: &str,
    entries: &[PickerEntry],
    default_selected_skills: &HashSet<String>,
) -> Result<Option<HashSet<String>>> {
    if entries.is_empty() {
        return Ok(Some(HashSet::new()));
    }

    let preset_rows = collect_preset_rows(entries);
    let chosen_members = if preset_rows.is_empty() {
        HashSet::new()
    } else {
        match select_preset_groups(&preset_rows, default_selected_skills)? {
            Some(chosen) => chosen,
            // User aborted at the group stage — cancel the whole flow.
            None => return Ok(None),
        }
    };

    select_skills(prompt, entries, &chosen_members, default_selected_skills)
}

/// Phase 1: wholesale group selection. Groups fully covered by the currently
/// selected skills start checked (so "confirm through" preserves the status
/// quo). Unchecking only means "don't add this group wholesale" — final
/// membership is always decided in phase 2.
fn select_preset_groups(
    preset_rows: &[PresetRow],
    default_selected_skills: &HashSet<String>,
) -> Result<Option<HashSet<String>>> {
    let labels: Vec<String> = preset_rows
        .iter()
        .map(|row| {
            let selected = row
                .skill_ids
                .iter()
                .filter(|skill_id| default_selected_skills.contains(skill_id.as_str()))
                .count();
            group_label(&row.label, selected)
        })
        .collect();
    let defaults: Vec<usize> = preset_rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            row.skill_ids
                .iter()
                .all(|skill_id| default_selected_skills.contains(skill_id.as_str()))
                .then_some(index)
        })
        .collect();

    match prompt_multiselect(
        &t!("skills.manager.select_presets"),
        labels,
        &defaults,
        true,
        // Group labels are plain text; the library default substring/fuzzy
        // scorer is enough for this phase.
        MultiSelect::<String>::DEFAULT_SCORER,
    )? {
        Some(selected) => Ok(Some(
            selected
                .into_iter()
                .flat_map(|list_option| preset_rows[list_option.index].skill_ids.clone())
                .collect(),
        )),
        None => Ok(None),
    }
}

/// Surface each group's current selection count, e.g.
/// `dakesan (2 skills, 1 selected)` — silent when nothing is selected.
fn group_label(label: &str, selected: usize) -> String {
    if selected == 0 {
        return label.to_string();
    }
    match label.strip_suffix(')') {
        Some(head) => format!("{head}, {selected} selected)"),
        None => format!("{label} ({selected} selected)"),
    }
}

/// Phase 2: the authoritative skill editor. Pre-checks the union of chosen
/// group members and the previously selected set.
fn select_skills(
    prompt: &str,
    entries: &[PickerEntry],
    chosen_members: &HashSet<String>,
    default_selected_skills: &HashSet<String>,
) -> Result<Option<HashSet<String>>> {
    let options = build_options(entries);
    let effective_defaults: HashSet<String> = chosen_members
        .iter()
        .cloned()
        .chain(default_selected_skills.iter().cloned())
        .collect();
    let defaults: Vec<usize> = options
        .iter()
        .enumerate()
        .filter_map(|(index, option)| {
            effective_defaults
                .contains(option.skill_id.as_str())
                .then_some(index)
        })
        .collect();

    match prompt_multiselect(prompt, options, &defaults, false, &score_skill_option)? {
        Some(selected) => Ok(Some(
            selected
                .into_iter()
                .map(|list_option| list_option.value.skill_id)
                .collect(),
        )),
        None => Ok(None),
    }
}

/// Case-insensitive scorer over both the display label and the skill id, so
/// filtering by a bare id (`ruff`) or a repo-flavoured id
/// (`dakesan.marimo-editor`) finds the row even if the label differs.
/// Label-prefix matches outrank id matches, which outrank label substrings.
fn score_skill_option(
    filter: &str,
    option: &PickerOption,
    _text: &str,
    _index: usize,
) -> Option<i64> {
    let needle = filter.to_lowercase();
    if needle.is_empty() {
        return Some(0);
    }
    let label = option.label.to_lowercase();
    let id = option.skill_id.to_lowercase();
    if label.starts_with(&needle) {
        Some(300)
    } else if id.contains(&needle) {
        Some(200)
    } else if label.contains(&needle) {
        Some(100)
    } else {
        None
    }
}

fn prompt_multiselect<'a, T: Display>(
    prompt: &'a str,
    options: Vec<T>,
    defaults: &'a [usize],
    group_phase: bool,
    scorer: Scorer<'a, T>,
) -> Result<Option<Vec<ListOption<T>>>> {
    let help_message = if group_phase {
        PRESET_HELP_MESSAGE
    } else {
        SKILL_HELP_MESSAGE
    };
    let mut select = MultiSelect::new(prompt, options)
        .with_default(defaults)
        .with_page_size(PAGE_SIZE)
        .with_vim_mode(true)
        .with_scorer(scorer)
        .with_help_message(help_message);
    if !group_phase {
        // Answered summary: the default comma-list gets unwieldy for many
        // skills — a count is enough (details were visible in the prompt).
        select = select.with_formatter(&|selections| format!("{} selected", selections.len()));
    }
    match select.raw_prompt_skippable() {
        Ok(selection) => Ok(selection),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Flatten preset/skill entries into a stable, deduplicated option list.
///
/// Preset-grouped skills come first (in preset order); skills not covered by
/// any preset form a trailing group sorted by label (mirroring the historical
/// tree picker's fallback "other" group).
fn build_options(entries: &[PickerEntry]) -> Vec<PickerOption> {
    let skill_labels: HashMap<&str, &str> = entries
        .iter()
        .filter_map(|entry| match &entry.kind {
            PickerEntryKind::Skill { skill_id } => Some((skill_id.as_str(), entry.label.as_str())),
            PickerEntryKind::Preset { .. } => None,
        })
        .collect();

    let mut options: Vec<PickerOption> = Vec::new();
    let mut covered: HashSet<&str> = HashSet::new();

    for entry in entries {
        let PickerEntryKind::Preset { skill_ids } = &entry.kind else {
            continue;
        };
        let mut seen = HashSet::new();
        for skill_id in skill_ids {
            if seen.insert(skill_id.as_str()) && covered.insert(skill_id.as_str()) {
                options.push(PickerOption {
                    skill_id: skill_id.clone(),
                    label: skill_labels
                        .get(skill_id.as_str())
                        .map(|label| (*label).to_string())
                        .unwrap_or_else(|| skill_id.clone()),
                });
            }
        }
    }

    let mut uncovered: Vec<&str> = skill_labels
        .keys()
        .copied()
        .filter(|skill_id| !covered.contains(skill_id))
        .collect();
    uncovered.sort_by(|a, b| {
        let a_label = skill_labels.get(a).copied().unwrap_or(a);
        let b_label = skill_labels.get(b).copied().unwrap_or(b);
        a_label.cmp(b_label).then_with(|| a.cmp(b))
    });

    options.extend(uncovered.into_iter().map(|skill_id| {
        PickerOption {
            label: skill_labels
                .get(skill_id)
                .map(|label| (*label).to_string())
                .unwrap_or_else(|| skill_id.to_string()),
            skill_id: skill_id.to_string(),
        }
    }));

    options
}

/// Preset groups in entry order, with per-group member deduplication.
fn collect_preset_rows(entries: &[PickerEntry]) -> Vec<PresetRow> {
    let mut rows: Vec<PresetRow> = Vec::new();
    for entry in entries {
        let PickerEntryKind::Preset { skill_ids } = &entry.kind else {
            continue;
        };
        let mut row = PresetRow {
            label: entry.label.clone(),
            skill_ids: Vec::new(),
        };
        let mut seen = HashSet::new();
        for skill_id in skill_ids {
            if seen.insert(skill_id.clone()) {
                row.skill_ids.push(skill_id.clone());
            }
        }
        if !row.skill_ids.is_empty() {
            rows.push(row);
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(label: &str, skill_ids: &[&str]) -> PickerEntry {
        PickerEntry {
            label: label.to_string(),
            kind: PickerEntryKind::Preset {
                skill_ids: skill_ids.iter().map(|id| (*id).to_string()).collect(),
            },
        }
    }

    fn skill(label: &str, skill_id: &str) -> PickerEntry {
        PickerEntry {
            label: label.to_string(),
            kind: PickerEntryKind::Skill {
                skill_id: skill_id.to_string(),
            },
        }
    }

    fn sample_entries() -> Vec<PickerEntry> {
        vec![
            preset("dakesan (2 skills)", &["marimo-editor", "marimo-inspect"]),
            skill("marimo-editor (dakesan.marimo-editor)", "marimo-editor"),
            skill("marimo-inspect (dakesan.marimo-inspect)", "marimo-inspect"),
            preset("astral-sh (1 skills)", &["ruff"]),
            skill("ruff (astral-sh.ruff)", "ruff"),
        ]
    }

    #[test]
    fn group_label_appends_selected_count_into_parentheses() {
        assert_eq!(
            group_label("dakesan (2 skills)", 1),
            "dakesan (2 skills, 1 selected)"
        );
        assert_eq!(
            group_label("dakesan (2 skills)", 2),
            "dakesan (2 skills, 2 selected)"
        );
    }

    #[test]
    fn group_label_stays_silent_when_nothing_is_selected() {
        assert_eq!(group_label("dakesan (2 skills)", 0), "dakesan (2 skills)");
    }

    #[test]
    fn group_label_falls_back_to_suffix_without_parentheses() {
        assert_eq!(group_label("dakesan", 1), "dakesan (1 selected)");
    }

    #[test]
    fn collect_preset_rows_keeps_order_and_dedupes_members() {
        let entries = vec![
            preset("p1 (2 skills)", &["a", "a", "b"]),
            skill("a (a.dir)", "a"),
            preset("p2 (1 skills)", &["c"]),
        ];
        let rows = collect_preset_rows(&entries);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].label, "p1 (2 skills)");
        assert_eq!(rows[0].skill_ids, vec!["a", "b"]);
        assert_eq!(rows[1].skill_ids, vec!["c"]);
    }

    #[test]
    fn collect_preset_rows_skips_empty_groups() {
        let entries = vec![preset("empty (0 skills)", &[])];
        assert!(collect_preset_rows(&entries).is_empty());
    }

    #[test]
    fn phase1_default_rule_groups_fully_defaulted_are_prechecked() {
        let defaults: HashSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        let entries = vec![
            preset("full (2 skills)", &["a", "b"]),
            preset("partial (2 skills)", &["a", "c"]),
        ];
        let rows = collect_preset_rows(&entries);
        let fully: Vec<bool> = rows
            .iter()
            .map(|row| {
                row.skill_ids
                    .iter()
                    .all(|skill_id| defaults.contains(skill_id))
            })
            .collect();
        assert_eq!(fully, vec![true, false]);
    }

    #[test]
    fn phase2_defaults_merge_chosen_members_with_previous_selection() {
        // Chosen group brings "b"; previous selection had "a"; "c" untouched.
        let chosen: HashSet<String> = ["b".to_string()].into_iter().collect();
        let defaults: HashSet<String> = ["a".to_string()].into_iter().collect();
        let merged: HashSet<String> = chosen
            .iter()
            .cloned()
            .chain(defaults.iter().cloned())
            .collect();
        assert!(merged.contains("a") && merged.contains("b"));
        assert!(!merged.contains("c"));
    }

    #[test]
    fn build_options_dedupes_and_keeps_preset_order() {
        let options = build_options(&sample_entries());
        let ids: Vec<&str> = options.iter().map(|o| o.skill_id.as_str()).collect();
        assert_eq!(ids, vec!["marimo-editor", "marimo-inspect", "ruff"]);
    }

    #[test]
    fn build_options_appends_uncovered_skills_sorted_by_label() {
        let entries = vec![
            preset("only-a (1 skills)", &["a"]),
            skill("b (b.dir)", "b"),
            skill("a (a.dir)", "a"),
        ];
        let options = build_options(&entries);
        let ids: Vec<&str> = options.iter().map(|o| o.skill_id.as_str()).collect();
        // "a" is covered by the preset; standalone "b" trails it.
        assert_eq!(ids, vec!["a", "b"]);
    }

    #[test]
    fn build_options_prefers_skill_entry_labels_over_ids() {
        let entries = vec![preset("p (1 skills)", &["x"]), skill("X skill (p.x)", "x")];
        let options = build_options(&entries);
        assert_eq!(options[0].label, "X skill (p.x)");
    }

    #[test]
    fn build_options_falls_back_to_skill_id_without_label_entry() {
        let entries = vec![preset("p (1 skills)", &["mystery"])];
        let options = build_options(&entries);
        assert_eq!(options[0].label, "mystery");
    }

    #[test]
    fn empty_entries_short_circuits_without_prompting() {
        let defaults = HashSet::new();
        let picked = pick("prompt", &[], &defaults).unwrap();
        assert_eq!(picked, Some(HashSet::new()));
    }

    fn option(skill_id: &str, label: &str) -> PickerOption {
        PickerOption {
            skill_id: skill_id.to_string(),
            label: label.to_string(),
        }
    }

    #[test]
    fn scorer_matches_label_substring_case_insensitively() {
        // Id does not contain "editor" — only the label does.
        let option = option("dakesan.marimo", "Marimo Editor");
        assert_eq!(score_skill_option("editor", &option, "", 0), Some(100));
        assert_eq!(score_skill_option("MARIMO", &option, "", 0), Some(300)); // label prefix
    }

    #[test]
    fn scorer_matches_bare_skill_id() {
        // Label carries context, but the bare id must be found too.
        let option = option(
            "dakesan.marimo-editor",
            "marimo-editor (dakesan.marimo-editor)",
        );
        assert_eq!(score_skill_option("dakesan", &option, "", 0), Some(200));
    }

    #[test]
    fn scorer_ranks_label_prefix_over_id_over_substring() {
        let prefix = option("x.beta", "alpha tool");
        let id_match = option("x.alpha", "beta tool");
        let substring = option("x.gamma", "the alphalist tool");
        assert_eq!(score_skill_option("alpha", &prefix, "", 0), Some(300));
        assert_eq!(score_skill_option("alpha", &id_match, "", 0), Some(200));
        assert_eq!(score_skill_option("alpha", &substring, "", 0), Some(100));
    }

    #[test]
    fn scorer_hides_non_matches_and_shows_everything_on_empty_filter() {
        let option = option("dakesan.marimo-editor", "Marimo Editor");
        assert_eq!(score_skill_option("ruff", &option, "", 0), None);
        assert_eq!(score_skill_option("", &option, "", 0), Some(0));
    }
}

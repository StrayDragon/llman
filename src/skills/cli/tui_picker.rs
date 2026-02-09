use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

const LIST_SCROLL_PADDING_ROWS: usize = 2;

#[derive(Clone, Debug)]
pub enum TuiEntryKind {
    Preset { skill_ids: Vec<String> },
    Skill { skill_id: String },
}

#[derive(Clone, Debug)]
pub struct TuiEntry {
    pub label: String,
    pub kind: TuiEntryKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionState {
    Unchecked,
    Checked,
    Partial,
}

#[derive(Clone, Copy)]
enum RowRef {
    Preset(usize),
    Skill { preset_idx: usize, skill_idx: usize },
}

#[derive(Clone)]
struct SkillNode {
    skill_id: String,
    label: String,
}

#[derive(Clone)]
struct PresetNode {
    label: String,
    skill_ids: Vec<String>,
    children: Vec<SkillNode>,
    expanded: bool,
}

struct TerminalRestoreGuard;

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

pub fn pick(
    prompt: &str,
    entries: &[TuiEntry],
    default_selected_skills: &HashSet<String>,
) -> Result<Option<HashSet<String>>> {
    if entries.is_empty() {
        return Ok(Some(HashSet::new()));
    }

    let mut terminal = ratatui::try_init()?;
    let _restore = TerminalRestoreGuard;

    let mut app = PickerState::new(entries.to_vec(), default_selected_skills);

    loop {
        terminal.draw(|frame| app.draw(frame, prompt))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.search_mode {
            match key.code {
                KeyCode::Esc => {
                    if app.filter_query.is_empty() {
                        return Ok(None);
                    }
                    app.clear_filter();
                    app.set_search_mode(false);
                }
                KeyCode::Enter => app.set_search_mode(false),
                KeyCode::Backspace => app.pop_filter_char(),
                KeyCode::Char(c) => app.append_filter_char(c),
                _ => {}
            }
            continue;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => app.move_up(),
            KeyCode::Down | KeyCode::Char('j') => app.move_down(),
            KeyCode::Char(' ') => app.toggle_current(),
            KeyCode::Right | KeyCode::Char('l') => app.expand_current(),
            KeyCode::Left | KeyCode::Char('h') => app.collapse_current(),
            KeyCode::Char('/') => app.set_search_mode(true),
            KeyCode::Char('a') => app.select_all_skills(),
            KeyCode::Char('n') => app.clear_all(),
            KeyCode::Enter => return Ok(Some(app.selected_skills())),
            KeyCode::Esc => return Ok(None),
            _ => {}
        }
    }
}

#[derive(Clone)]
struct PickerState {
    presets: Vec<PresetNode>,
    cursor: usize,
    selected_skills: HashSet<String>,
    filter_query: String,
    search_mode: bool,
}

impl PickerState {
    fn new(entries: Vec<TuiEntry>, default_selected_skills: &HashSet<String>) -> Self {
        let presets = build_presets(entries);
        let known_skill_ids: HashSet<String> = presets
            .iter()
            .flat_map(|preset| preset.skill_ids.iter().cloned())
            .collect();
        let selected_skills = default_selected_skills
            .iter()
            .filter(|skill_id| known_skill_ids.contains(skill_id.as_str()))
            .cloned()
            .collect();

        Self {
            presets,
            cursor: 0,
            selected_skills,
            filter_query: String::new(),
            search_mode: false,
        }
    }

    fn selected_skills(&self) -> HashSet<String> {
        self.selected_skills.clone()
    }

    fn draw(&self, frame: &mut ratatui::Frame, prompt: &str) {
        let rows = self.visible_rows();
        let items: Vec<ListItem> = rows
            .iter()
            .map(|row| match row {
                RowRef::Preset(preset_idx) => {
                    let preset = &self.presets[*preset_idx];
                    let marker = selection_marker(self.preset_state(*preset_idx));
                    let branch = if preset.expanded { "▾" } else { "▸" };
                    ListItem::new(Line::from(format!(
                        "{} {} {}",
                        marker, branch, preset.label
                    )))
                }
                RowRef::Skill {
                    preset_idx,
                    skill_idx,
                } => {
                    let skill = &self.presets[*preset_idx].children[*skill_idx];
                    let marker = if self.selected_skills.contains(skill.skill_id.as_str()) {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    ListItem::new(Line::from(format!("  {} {}", marker, skill.label)))
                }
            })
            .collect();

        let chunks = Layout::vertical([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

        let mut state = ListState::default();
        if rows.is_empty() {
            state.select(None);
        } else {
            state.select(Some(self.cursor.min(rows.len().saturating_sub(1))));
        }

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(prompt))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .scroll_padding(LIST_SCROLL_PADDING_ROWS)
            .highlight_symbol("> ");
        frame.render_stateful_widget(list, chunks[0], &mut state);

        let filter_value = if self.filter_query.is_empty() {
            "(none)".to_string()
        } else {
            self.filter_query.clone()
        };
        let filter_line = if self.search_mode {
            format!("Filter: {}_", filter_value)
        } else {
            format!("Filter: {}", filter_value)
        };
        let filter =
            Paragraph::new(Line::from(filter_line)).style(Style::default().fg(Color::DarkGray));
        frame.render_widget(filter, chunks[1]);

        let help = Paragraph::new(Line::from(
            "↑↓ move · space toggle · →/l expand · ←/h collapse · / search · a all · n none · Enter confirm · Esc cancel",
        ))
        .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(help, chunks[2]);
    }

    fn visible_rows(&self) -> Vec<RowRef> {
        let Some(query) = normalized_query(&self.filter_query) else {
            return self.unfiltered_rows();
        };

        let mut rows = Vec::new();
        for (preset_idx, preset) in self.presets.iter().enumerate() {
            let preset_match = text_matches_query(&preset.label, &query);
            let matching_children = preset
                .children
                .iter()
                .enumerate()
                .filter_map(|(skill_idx, skill)| {
                    if text_matches_query(&skill.label, &query)
                        || text_matches_query(&skill.skill_id, &query)
                    {
                        Some(skill_idx)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if !preset_match && matching_children.is_empty() {
                continue;
            }

            rows.push(RowRef::Preset(preset_idx));

            if !matching_children.is_empty() {
                rows.extend(
                    matching_children
                        .into_iter()
                        .map(|skill_idx| RowRef::Skill {
                            preset_idx,
                            skill_idx,
                        }),
                );
            } else if preset_match && preset.expanded {
                rows.extend((0..preset.children.len()).map(|skill_idx| RowRef::Skill {
                    preset_idx,
                    skill_idx,
                }));
            }
        }

        rows
    }

    fn unfiltered_rows(&self) -> Vec<RowRef> {
        let mut rows = Vec::new();
        for (preset_idx, preset) in self.presets.iter().enumerate() {
            rows.push(RowRef::Preset(preset_idx));
            if preset.expanded {
                for (skill_idx, _) in preset.children.iter().enumerate() {
                    rows.push(RowRef::Skill {
                        preset_idx,
                        skill_idx,
                    });
                }
            }
        }
        rows
    }

    fn move_up(&mut self) {
        let len = self.visible_rows().len();
        if len == 0 {
            return;
        }
        self.cursor = if self.cursor == 0 {
            len - 1
        } else {
            self.cursor - 1
        };
    }

    fn move_down(&mut self) {
        let len = self.visible_rows().len();
        if len == 0 {
            return;
        }
        self.cursor = (self.cursor + 1) % len;
    }

    fn toggle_current(&mut self) {
        let rows = self.visible_rows();
        let Some(row) = rows.get(self.cursor).copied() else {
            return;
        };

        match row {
            RowRef::Preset(preset_idx) => self.toggle_preset(preset_idx),
            RowRef::Skill {
                preset_idx,
                skill_idx,
            } => self.toggle_skill(preset_idx, skill_idx),
        }
    }

    fn expand_current(&mut self) {
        let rows = self.visible_rows();
        let Some(row) = rows.get(self.cursor).copied() else {
            return;
        };

        if let RowRef::Preset(preset_idx) = row
            && let Some(preset) = self.presets.get_mut(preset_idx)
        {
            preset.expanded = true;
            self.clamp_cursor();
        }
    }

    fn collapse_current(&mut self) {
        let rows = self.visible_rows();
        let Some(row) = rows.get(self.cursor).copied() else {
            return;
        };

        match row {
            RowRef::Preset(preset_idx) => {
                if let Some(preset) = self.presets.get_mut(preset_idx) {
                    preset.expanded = false;
                }
            }
            RowRef::Skill { preset_idx, .. } => {
                if let Some(preset) = self.presets.get_mut(preset_idx) {
                    preset.expanded = false;
                }
                self.cursor = self.row_index_for_preset(preset_idx);
            }
        }

        let len = self.visible_rows().len();
        if len > 0 {
            self.cursor = self.cursor.min(len - 1);
        }
    }

    fn clamp_cursor(&mut self) {
        let len = self.visible_rows().len();
        if len == 0 {
            self.cursor = 0;
        } else {
            self.cursor = self.cursor.min(len - 1);
        }
    }

    fn set_search_mode(&mut self, enabled: bool) {
        self.search_mode = enabled;
    }

    fn append_filter_char(&mut self, c: char) {
        if c.is_control() {
            return;
        }
        self.filter_query.push(c);
        self.clamp_cursor();
    }

    fn pop_filter_char(&mut self) {
        self.filter_query.pop();
        self.clamp_cursor();
    }

    fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.clamp_cursor();
    }

    fn row_index_for_preset(&self, target_preset_idx: usize) -> usize {
        let rows = self.visible_rows();
        rows.iter()
            .position(
                |row| matches!(row, RowRef::Preset(preset_idx) if *preset_idx == target_preset_idx),
            )
            .unwrap_or(0)
    }

    fn toggle_preset(&mut self, preset_idx: usize) {
        let Some(preset) = self.presets.get(preset_idx) else {
            return;
        };

        match self.preset_state(preset_idx) {
            SelectionState::Checked => {
                for skill_id in &preset.skill_ids {
                    self.selected_skills.remove(skill_id);
                }
            }
            SelectionState::Unchecked | SelectionState::Partial => {
                for skill_id in &preset.skill_ids {
                    self.selected_skills.insert(skill_id.clone());
                }
            }
        }
    }

    fn toggle_skill(&mut self, preset_idx: usize, skill_idx: usize) {
        let Some(skill) = self
            .presets
            .get(preset_idx)
            .and_then(|preset| preset.children.get(skill_idx))
        else {
            return;
        };

        if self.selected_skills.contains(skill.skill_id.as_str()) {
            self.selected_skills.remove(skill.skill_id.as_str());
        } else {
            self.selected_skills.insert(skill.skill_id.clone());
        }
    }

    fn select_all_skills(&mut self) {
        for preset in &self.presets {
            for skill_id in &preset.skill_ids {
                self.selected_skills.insert(skill_id.clone());
            }
        }
    }

    fn clear_all(&mut self) {
        self.selected_skills.clear();
    }

    fn preset_state(&self, preset_idx: usize) -> SelectionState {
        let Some(preset) = self.presets.get(preset_idx) else {
            return SelectionState::Unchecked;
        };

        if preset.skill_ids.is_empty() {
            return SelectionState::Unchecked;
        }

        let selected_count = preset
            .skill_ids
            .iter()
            .filter(|skill_id| self.selected_skills.contains(skill_id.as_str()))
            .count();

        if selected_count == 0 {
            SelectionState::Unchecked
        } else if selected_count == preset.skill_ids.len() {
            SelectionState::Checked
        } else {
            SelectionState::Partial
        }
    }
}

fn build_presets(entries: Vec<TuiEntry>) -> Vec<PresetNode> {
    let mut skill_labels = HashMap::<String, String>::new();
    for entry in &entries {
        if let TuiEntryKind::Skill { skill_id } = &entry.kind {
            skill_labels
                .entry(skill_id.clone())
                .or_insert_with(|| entry.label.clone());
        }
    }

    let mut presets = Vec::new();
    let mut covered_skill_ids = HashSet::<String>::new();

    for entry in &entries {
        let TuiEntryKind::Preset { skill_ids } = &entry.kind else {
            continue;
        };

        let mut uniq_skill_ids = Vec::new();
        let mut seen = HashSet::new();
        for skill_id in skill_ids {
            if seen.insert(skill_id.clone()) {
                uniq_skill_ids.push(skill_id.clone());
                covered_skill_ids.insert(skill_id.clone());
            }
        }

        let children = uniq_skill_ids
            .iter()
            .map(|skill_id| SkillNode {
                skill_id: skill_id.clone(),
                label: skill_labels
                    .get(skill_id)
                    .cloned()
                    .unwrap_or_else(|| skill_id.clone()),
            })
            .collect();

        presets.push(PresetNode {
            label: entry.label.clone(),
            skill_ids: uniq_skill_ids,
            children,
            expanded: true,
        });
    }

    let mut uncovered_ids = skill_labels
        .keys()
        .filter(|skill_id| !covered_skill_ids.contains(skill_id.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if presets.is_empty() && !skill_labels.is_empty() {
        uncovered_ids = skill_labels.keys().cloned().collect();
    }

    if !uncovered_ids.is_empty() {
        uncovered_ids.sort_by(|a, b| {
            let a_label = skill_labels
                .get(a)
                .map(String::as_str)
                .unwrap_or(a.as_str());
            let b_label = skill_labels
                .get(b)
                .map(String::as_str)
                .unwrap_or(b.as_str());
            a_label.cmp(b_label).then_with(|| a.cmp(b))
        });
        let children = uncovered_ids
            .iter()
            .map(|skill_id| SkillNode {
                skill_id: skill_id.clone(),
                label: skill_labels
                    .get(skill_id)
                    .cloned()
                    .unwrap_or_else(|| skill_id.clone()),
            })
            .collect::<Vec<_>>();

        let label = if presets.is_empty() {
            format!("skills ({} skills)", uncovered_ids.len())
        } else {
            format!("other ({} skills)", uncovered_ids.len())
        };

        presets.push(PresetNode {
            label,
            skill_ids: uncovered_ids,
            children,
            expanded: true,
        });
    }

    presets
}

fn selection_marker(state: SelectionState) -> &'static str {
    match state {
        SelectionState::Unchecked => "[ ]",
        SelectionState::Checked => "[x]",
        SelectionState::Partial => "[-]",
    }
}

fn normalized_query(query: &str) -> Option<String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_lowercase())
}

fn text_matches_query(text: &str, query: &str) -> bool {
    text.to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<TuiEntry> {
        vec![
            TuiEntry {
                label: "dakesan (2 skills)".to_string(),
                kind: TuiEntryKind::Preset {
                    skill_ids: vec!["marimo-editor".to_string(), "marimo-inspect".to_string()],
                },
            },
            TuiEntry {
                label: "marimo-editor (dakesan.marimo-editor)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "marimo-editor".to_string(),
                },
            },
            TuiEntry {
                label: "marimo-inspect (dakesan.marimo-inspect)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "marimo-inspect".to_string(),
                },
            },
            TuiEntry {
                label: "astral-sh (1 skills)".to_string(),
                kind: TuiEntryKind::Preset {
                    skill_ids: vec!["ruff".to_string()],
                },
            },
            TuiEntry {
                label: "ruff (astral-sh.ruff)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "ruff".to_string(),
                },
            },
        ]
    }

    #[test]
    fn preset_state_partial_when_only_some_skills_selected() {
        let entries = vec![
            TuiEntry {
                label: "dakesan (2 skills)".to_string(),
                kind: TuiEntryKind::Preset {
                    skill_ids: vec!["a".to_string(), "b".to_string()],
                },
            },
            TuiEntry {
                label: "a (a.dir)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "a".to_string(),
                },
            },
            TuiEntry {
                label: "b (b.dir)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "b".to_string(),
                },
            },
        ];

        let mut defaults = HashSet::new();
        defaults.insert("a".to_string());
        let state = PickerState::new(entries, &defaults);

        assert_eq!(state.preset_state(0), SelectionState::Partial);
    }

    #[test]
    fn build_presets_includes_fallback_other_group_for_uncovered_skills() {
        let entries = vec![
            TuiEntry {
                label: "only-a (1 skills)".to_string(),
                kind: TuiEntryKind::Preset {
                    skill_ids: vec!["a".to_string()],
                },
            },
            TuiEntry {
                label: "a (a.dir)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "a".to_string(),
                },
            },
            TuiEntry {
                label: "b (b.dir)".to_string(),
                kind: TuiEntryKind::Skill {
                    skill_id: "b".to_string(),
                },
            },
        ];

        let presets = build_presets(entries);
        assert_eq!(presets.len(), 2);
        assert!(
            presets
                .iter()
                .any(|preset| preset.label.starts_with("other"))
        );
    }

    #[test]
    fn visible_rows_filter_matches_group_and_children() {
        let defaults = HashSet::new();
        let mut state = PickerState::new(sample_entries(), &defaults);

        state.filter_query = "dakesan".to_string();
        let rows = state.visible_rows();

        assert_eq!(rows.len(), 3);
        assert!(matches!(rows[0], RowRef::Preset(0)));
        assert!(matches!(rows[1], RowRef::Skill { preset_idx: 0, .. }));
        assert!(matches!(rows[2], RowRef::Skill { preset_idx: 0, .. }));
    }

    #[test]
    fn visible_rows_filter_matches_single_skill_keeps_parent_visible() {
        let defaults = HashSet::new();
        let mut state = PickerState::new(sample_entries(), &defaults);

        state.filter_query = "ruff".to_string();
        let rows = state.visible_rows();

        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], RowRef::Preset(1)));
        assert!(matches!(
            rows[1],
            RowRef::Skill {
                preset_idx: 1,
                skill_idx: 0
            }
        ));
    }

    #[test]
    fn append_and_clear_filter_clamps_cursor() {
        let defaults = HashSet::new();
        let mut state = PickerState::new(sample_entries(), &defaults);

        state.cursor = 4;
        state.append_filter_char('r');
        state.append_filter_char('u');
        state.append_filter_char('f');
        state.append_filter_char('f');
        assert_eq!(state.filter_query, "ruff");
        assert_eq!(state.cursor, 1);

        state.clear_filter();
        assert_eq!(state.filter_query, "");
        assert_eq!(state.cursor, 1);
    }
}

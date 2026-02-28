use crate::usage_stats::aggregate::{
    build_session_detail_view, build_sessions_view, build_summary_view, build_trend_view,
};
use crate::usage_stats::model::{SessionId, SessionRecord, ToolKind};
use crate::usage_stats::path_display::display_path;
use crate::usage_stats::query::{GroupBy, TimeRangeArgs, parse_time_range};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct StatsTuiScanRequest {
    pub cwd: std::path::PathBuf,
    pub group_by: GroupBy,
    pub range: TimeRangeArgs,
    pub limit: usize,
    pub verbose_paths: bool,
    pub with_breakdown: bool,
    pub include_sidechain: bool,
}

#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub label: &'static str,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub enum ScanMessage {
    Progress(ScanProgress),
    Done(Vec<SessionRecord>),
    Error(String),
}

pub type ScanFn = Arc<
    dyn Fn(StatsTuiScanRequest, Sender<ScanMessage>) -> Result<Vec<SessionRecord>> + Send + Sync,
>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatsTab {
    Overview,
    Trend,
    Sessions,
    SessionDetail,
}

impl StatsTab {
    fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Trend => "Trend",
            Self::Sessions => "Sessions",
            Self::SessionDetail => "Session",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Overview => Self::Trend,
            Self::Trend => Self::Sessions,
            Self::Sessions => Self::SessionDetail,
            Self::SessionDetail => Self::Overview,
        }
    }

    fn prev(self) -> Self {
        match self {
            Self::Overview => Self::SessionDetail,
            Self::Trend => Self::Overview,
            Self::Sessions => Self::Trend,
            Self::SessionDetail => Self::Sessions,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimePreset {
    All,
    Last7d,
    Last30d,
    Last90d,
    Custom,
}

impl TimePreset {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Last7d => "7d",
            Self::Last30d => "30d",
            Self::Last90d => "90d",
            Self::Custom => "Custom",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterField {
    Preset,
    Since,
    Until,
    GroupBy,
    Breakdown,
    Sidechain,
    Apply,
    Cancel,
}

impl FilterField {
    fn all_for(tool: ToolKind) -> Vec<FilterField> {
        let mut fields = vec![
            FilterField::Preset,
            FilterField::Since,
            FilterField::Until,
            FilterField::GroupBy,
        ];

        match tool {
            ToolKind::Codex => fields.push(FilterField::Breakdown),
            ToolKind::ClaudeCode => fields.push(FilterField::Sidechain),
            ToolKind::Cursor => {}
        }

        fields.push(FilterField::Apply);
        fields.push(FilterField::Cancel);
        fields
    }
}

#[derive(Clone, Debug)]
struct FilterFormState {
    active_field_idx: usize,
    fields: Vec<FilterField>,
    preset: TimePreset,
    since: String,
    until: String,
    group_by: GroupBy,
    with_breakdown: bool,
    include_sidechain: bool,
    error: Option<String>,
}

impl FilterFormState {
    fn new(tool: ToolKind, request: &StatsTuiScanRequest) -> Self {
        let preset = match request.range.last.as_deref() {
            Some("7d") => TimePreset::Last7d,
            Some("30d") => TimePreset::Last30d,
            Some("90d") => TimePreset::Last90d,
            Some(_) => TimePreset::Custom,
            None => {
                if request.range.since.is_some() || request.range.until.is_some() {
                    TimePreset::Custom
                } else {
                    TimePreset::All
                }
            }
        };

        Self {
            active_field_idx: 0,
            fields: FilterField::all_for(tool),
            preset,
            since: request.range.since.clone().unwrap_or_default(),
            until: request.range.until.clone().unwrap_or_default(),
            group_by: request.group_by,
            with_breakdown: request.with_breakdown,
            include_sidechain: request.include_sidechain,
            error: None,
        }
    }

    fn active_field(&self) -> FilterField {
        self.fields[self.active_field_idx]
    }

    fn move_next(&mut self) {
        self.active_field_idx = (self.active_field_idx + 1).min(self.fields.len() - 1);
    }

    fn move_prev(&mut self) {
        self.active_field_idx = self.active_field_idx.saturating_sub(1);
    }

    fn cycle_preset_left(&mut self) {
        self.preset = match self.preset {
            TimePreset::All => TimePreset::Custom,
            TimePreset::Last7d => TimePreset::All,
            TimePreset::Last30d => TimePreset::Last7d,
            TimePreset::Last90d => TimePreset::Last30d,
            TimePreset::Custom => TimePreset::Last90d,
        };
    }

    fn cycle_preset_right(&mut self) {
        self.preset = match self.preset {
            TimePreset::All => TimePreset::Last7d,
            TimePreset::Last7d => TimePreset::Last30d,
            TimePreset::Last30d => TimePreset::Last90d,
            TimePreset::Last90d => TimePreset::Custom,
            TimePreset::Custom => TimePreset::All,
        };
    }

    fn cycle_group_by_left(&mut self) {
        self.group_by = match self.group_by {
            GroupBy::Day => GroupBy::Month,
            GroupBy::Week => GroupBy::Day,
            GroupBy::Month => GroupBy::Week,
        };
    }

    fn cycle_group_by_right(&mut self) {
        self.group_by = match self.group_by {
            GroupBy::Day => GroupBy::Week,
            GroupBy::Week => GroupBy::Month,
            GroupBy::Month => GroupBy::Day,
        };
    }

    fn append_char(&mut self, c: char) {
        if self.preset != TimePreset::Custom {
            return;
        }
        match self.active_field() {
            FilterField::Since => self.since.push(c),
            FilterField::Until => self.until.push(c),
            _ => {}
        }
    }

    fn pop_char(&mut self) {
        if self.preset != TimePreset::Custom {
            return;
        }
        match self.active_field() {
            FilterField::Since => {
                self.since.pop();
            }
            FilterField::Until => {
                self.until.pop();
            }
            _ => {}
        }
    }

    fn build_range_args(&self) -> TimeRangeArgs {
        match self.preset {
            TimePreset::All => TimeRangeArgs::default(),
            TimePreset::Last7d => TimeRangeArgs {
                last: Some("7d".to_string()),
                ..TimeRangeArgs::default()
            },
            TimePreset::Last30d => TimeRangeArgs {
                last: Some("30d".to_string()),
                ..TimeRangeArgs::default()
            },
            TimePreset::Last90d => TimeRangeArgs {
                last: Some("90d".to_string()),
                ..TimeRangeArgs::default()
            },
            TimePreset::Custom => TimeRangeArgs {
                since: (!self.since.trim().is_empty()).then(|| self.since.trim().to_string()),
                until: (!self.until.trim().is_empty()).then(|| self.until.trim().to_string()),
                last: None,
            },
        }
    }
}

#[derive(Clone, Debug)]
struct ScanResultCache {
    sessions_all: Vec<SessionRecord>,
    sessions_list: Vec<SessionRecord>,
    summary_lines: Vec<String>,
    trend_lines: Vec<String>,
}

#[derive(Clone, Debug)]
enum ScanStatus {
    Idle,
    Scanning(Option<ScanProgress>),
    Ready(ScanResultCache),
    Error(String),
}

struct TerminalRestoreGuard;

impl Drop for TerminalRestoreGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

pub fn run_stats_tui(tool: ToolKind, initial: StatsTuiScanRequest, scan_fn: ScanFn) -> Result<()> {
    let mut terminal = ratatui::try_init()?;
    let _restore = TerminalRestoreGuard;

    let mut app = StatsTuiApp::new(tool, initial, scan_fn);
    app.start_scan();

    loop {
        terminal.draw(|frame| app.draw(frame))?;

        app.drain_scan_messages();

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.handle_key(key.code)? {
            break;
        }
    }

    Ok(())
}

struct StatsTuiApp {
    tool: ToolKind,
    tab: StatsTab,
    request: StatsTuiScanRequest,
    scan_fn: ScanFn,
    scan_rx: Option<Receiver<ScanMessage>>,
    status: ScanStatus,
    filter_open: bool,
    filter: FilterFormState,
    sessions_selected: Option<usize>,
    session_detail_id: Option<SessionId>,
}

impl StatsTuiApp {
    fn new(tool: ToolKind, request: StatsTuiScanRequest, scan_fn: ScanFn) -> Self {
        let filter = FilterFormState::new(tool, &request);
        Self {
            tool,
            tab: StatsTab::Overview,
            request,
            scan_fn,
            scan_rx: None,
            status: ScanStatus::Idle,
            filter_open: false,
            filter,
            sessions_selected: Some(0),
            session_detail_id: None,
        }
    }

    fn start_scan(&mut self) {
        let (tx, rx) = channel::<ScanMessage>();
        self.scan_rx = Some(rx);
        self.status = ScanStatus::Scanning(None);

        let scan_fn = Arc::clone(&self.scan_fn);
        let request = self.request.clone();
        std::thread::spawn(move || {
            let result = (scan_fn)(request, tx.clone());
            match result {
                Ok(sessions) => {
                    let _ = tx.send(ScanMessage::Done(sessions));
                }
                Err(err) => {
                    let _ = tx.send(ScanMessage::Error(err.to_string()));
                }
            }
        });
    }

    fn drain_scan_messages(&mut self) {
        let Some(rx) = &self.scan_rx else {
            return;
        };
        while let Ok(msg) = rx.try_recv() {
            match msg {
                ScanMessage::Progress(progress) => {
                    self.status = ScanStatus::Scanning(Some(progress));
                }
                ScanMessage::Done(sessions) => {
                    self.status = ScanStatus::Ready(self.build_cache(&sessions));
                    self.sessions_selected = Some(0);
                    self.session_detail_id = None;
                }
                ScanMessage::Error(message) => {
                    self.status = ScanStatus::Error(message);
                }
            }
        }
    }

    fn build_cache(&self, sessions: &[SessionRecord]) -> ScanResultCache {
        let summary = build_summary_view(sessions);
        let trend = build_trend_view(sessions, self.request.group_by);
        let sessions_view = build_sessions_view(sessions, self.request.limit);

        let mut summary_lines = Vec::new();
        summary_lines.push(format!(
            "Sessions: {} (known tokens: {})",
            summary.coverage.total_sessions, summary.coverage.known_token_sessions
        ));
        summary_lines.push(format!(
            "Tokens (known-only): {}",
            summary.totals.tokens_total_known
        ));
        if let Some(v) = summary.totals.tokens_input_known {
            summary_lines.push(format!("  input: {v}"));
        }
        if let Some(v) = summary.totals.tokens_output_known {
            summary_lines.push(format!("  output: {v}"));
        }
        if let Some(v) = summary.totals.tokens_cache_known {
            summary_lines.push(format!("  cache: {v}"));
        }
        if let Some(v) = summary.totals.tokens_reasoning_known {
            summary_lines.push(format!("  reasoning: {v}"));
        }

        let mut trend_lines = Vec::new();
        trend_lines.push("bucket\tknown_tokens\tsessions(known/total)".to_string());
        for bucket in trend.buckets {
            trend_lines.push(format!(
                "{}\t{}\t{}/{}",
                bucket.label,
                bucket.totals.tokens_total_known,
                bucket.coverage.known_token_sessions,
                bucket.coverage.total_sessions
            ));
        }

        ScanResultCache {
            sessions_all: sessions.to_vec(),
            sessions_list: sessions_view.sessions,
            summary_lines,
            trend_lines,
        }
    }

    fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        if self.filter_open {
            return Ok(self.handle_filter_key(key));
        }

        match key {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Tab => self.tab = self.tab.next(),
            KeyCode::BackTab => self.tab = self.tab.prev(),
            KeyCode::Char('f') => self.open_filter(),
            KeyCode::Up | KeyCode::Char('k') => self.sessions_move(-1),
            KeyCode::Down | KeyCode::Char('j') => self.sessions_move(1),
            KeyCode::Enter => self.sessions_enter(),
            _ => {}
        }
        Ok(false)
    }

    fn open_filter(&mut self) {
        self.filter = FilterFormState::new(self.tool, &self.request);
        self.filter_open = true;
    }

    fn handle_filter_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.filter_open = false;
            }
            KeyCode::Up => self.filter.move_prev(),
            KeyCode::Down => self.filter.move_next(),
            KeyCode::Left => match self.filter.active_field() {
                FilterField::Preset => self.filter.cycle_preset_left(),
                FilterField::GroupBy => self.filter.cycle_group_by_left(),
                FilterField::Breakdown => self.filter.with_breakdown = !self.filter.with_breakdown,
                FilterField::Sidechain => {
                    self.filter.include_sidechain = !self.filter.include_sidechain;
                }
                _ => {}
            },
            KeyCode::Right => match self.filter.active_field() {
                FilterField::Preset => self.filter.cycle_preset_right(),
                FilterField::GroupBy => self.filter.cycle_group_by_right(),
                FilterField::Breakdown => self.filter.with_breakdown = !self.filter.with_breakdown,
                FilterField::Sidechain => {
                    self.filter.include_sidechain = !self.filter.include_sidechain;
                }
                _ => {}
            },
            KeyCode::Backspace => self.filter.pop_char(),
            KeyCode::Char(c) => self.filter.append_char(c),
            KeyCode::Enter => match self.filter.active_field() {
                FilterField::Apply if self.apply_filter_form() => {
                    self.filter_open = false;
                }
                FilterField::Cancel => self.filter_open = false,
                _ => {}
            },
            _ => {}
        }
        false
    }

    fn apply_filter_form(&mut self) -> bool {
        let range = self.filter.build_range_args();
        if let Err(err) = parse_time_range(&range, chrono::Utc::now()) {
            self.filter.error = Some(err.to_string());
            return false;
        }

        self.request.group_by = self.filter.group_by;
        self.request.range = range;
        self.request.with_breakdown = self.filter.with_breakdown;
        self.request.include_sidechain = self.filter.include_sidechain;

        self.start_scan();
        true
    }

    fn sessions_move(&mut self, delta: isize) {
        if self.tab != StatsTab::Sessions {
            return;
        }
        let ScanStatus::Ready(cache) = &self.status else {
            return;
        };
        if cache.sessions_list.is_empty() {
            self.sessions_selected = None;
            return;
        }
        let current = self.sessions_selected.unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, cache.sessions_list.len().saturating_sub(1) as isize);
        self.sessions_selected = Some(next as usize);
    }

    fn sessions_enter(&mut self) {
        if self.tab != StatsTab::Sessions {
            return;
        }
        let ScanStatus::Ready(cache) = &self.status else {
            return;
        };
        let Some(idx) = self.sessions_selected else {
            return;
        };
        let Some(session) = cache.sessions_list.get(idx) else {
            return;
        };
        self.session_detail_id = Some(session.id.clone());
        self.tab = StatsTab::SessionDetail;
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        let area = frame.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
            .split(area);

        self.draw_tabs(frame, chunks[0]);
        self.draw_body(frame, chunks[1]);

        if self.filter_open {
            self.draw_filter_modal(frame, area);
        }
    }

    fn draw_tabs(&self, frame: &mut ratatui::Frame, area: Rect) {
        let titles = [
            StatsTab::Overview.title(),
            StatsTab::Trend.title(),
            StatsTab::Sessions.title(),
            StatsTab::SessionDetail.title(),
        ]
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(Color::White))))
        .collect::<Vec<_>>();

        let selected = match self.tab {
            StatsTab::Overview => 0,
            StatsTab::Trend => 1,
            StatsTab::Sessions => 2,
            StatsTab::SessionDetail => 3,
        };

        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(tool_title(self.tool)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .select(selected);
        frame.render_widget(tabs, area);
    }

    fn draw_body(&self, frame: &mut ratatui::Frame, area: Rect) {
        match &self.status {
            ScanStatus::Idle => {
                frame.render_widget(
                    Paragraph::new("Idle").block(Block::default().borders(Borders::ALL)),
                    area,
                );
            }
            ScanStatus::Scanning(progress) => {
                let mut lines = vec!["Scanning…".to_string()];
                if let Some(progress) = progress {
                    lines.push(format!(
                        "{}: {}/{}",
                        progress.label, progress.done, progress.total
                    ));
                }
                frame.render_widget(
                    Paragraph::new(lines.join("\n")).block(Block::default().borders(Borders::ALL)),
                    area,
                );
            }
            ScanStatus::Error(message) => {
                frame.render_widget(
                    Paragraph::new(message.as_str())
                        .block(Block::default().borders(Borders::ALL).title("Error"))
                        .wrap(Wrap { trim: true }),
                    area,
                );
            }
            ScanStatus::Ready(cache) => match self.tab {
                StatsTab::Overview => {
                    frame.render_widget(
                        Paragraph::new(cache.summary_lines.join("\n"))
                            .block(Block::default().borders(Borders::ALL))
                            .wrap(Wrap { trim: true }),
                        area,
                    );
                }
                StatsTab::Trend => {
                    frame.render_widget(
                        Paragraph::new(cache.trend_lines.join("\n"))
                            .block(Block::default().borders(Borders::ALL))
                            .wrap(Wrap { trim: false }),
                        area,
                    );
                }
                StatsTab::Sessions => {
                    let items = cache
                        .sessions_list
                        .iter()
                        .enumerate()
                        .map(|(idx, session)| {
                            let end = session.end_ts.with_timezone(&chrono::Local);
                            let end = end.format("%Y-%m-%d %H:%M").to_string();
                            let tokens = session
                                .token_usage
                                .total
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "-".to_string());
                            let cwd = display_path(
                                &session.cwd,
                                &self.request.cwd,
                                self.request.verbose_paths,
                            );
                            let title = session.title.as_deref().unwrap_or("-");
                            let row = format!("{end}\t{tokens}\t{}\t{cwd}\t{title}", session.id);

                            let style = if self.sessions_selected == Some(idx) {
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default()
                            };
                            ListItem::new(Line::from(Span::styled(row, style)))
                        })
                        .collect::<Vec<_>>();

                    let list = List::new(items)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title("end\tknown_tokens\tid\tcwd\ttitle"),
                        )
                        .highlight_style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        );
                    frame.render_widget(list, area);
                }
                StatsTab::SessionDetail => {
                    let Some(id) = &self.session_detail_id else {
                        frame.render_widget(
                            Paragraph::new("Select a session from Sessions tab and press Enter.")
                                .block(Block::default().borders(Borders::ALL)),
                            area,
                        );
                        return;
                    };
                    let Some(view) = build_session_detail_view(&cache.sessions_all, id) else {
                        frame.render_widget(
                            Paragraph::new("Session not found.")
                                .block(Block::default().borders(Borders::ALL)),
                            area,
                        );
                        return;
                    };
                    let session = view.session;
                    let mut lines = Vec::new();
                    lines.push(format!("Id: {}", session.id));
                    lines.push(format!(
                        "End: {}",
                        session
                            .end_ts
                            .with_timezone(&chrono::Local)
                            .format("%Y-%m-%d %H:%M:%S")
                    ));
                    if let Some(start) = session.start_ts {
                        lines.push(format!(
                            "Start: {}",
                            start
                                .with_timezone(&chrono::Local)
                                .format("%Y-%m-%d %H:%M:%S")
                        ));
                    }
                    lines.push(format!(
                        "Cwd: {}",
                        display_path(&session.cwd, &self.request.cwd, self.request.verbose_paths)
                    ));
                    if let Some(title) = session.title {
                        lines.push(format!("Title: {title}"));
                    }
                    if let Some(total) = session.token_usage.total {
                        lines.push(format!("Tokens (known): total={total}"));
                    } else {
                        lines.push("Tokens: unknown".to_string());
                    }
                    if let Some(v) = session.token_usage.input {
                        lines.push(format!("  input={v}"));
                    }
                    if let Some(v) = session.token_usage.output {
                        lines.push(format!("  output={v}"));
                    }
                    if let Some(v) = session.token_usage.cache {
                        lines.push(format!("  cache={v}"));
                    }
                    if let Some(v) = session.token_usage.reasoning {
                        lines.push(format!("  reasoning={v}"));
                    }

                    frame.render_widget(
                        Paragraph::new(lines.join("\n"))
                            .block(Block::default().borders(Borders::ALL))
                            .wrap(Wrap { trim: true }),
                        area,
                    );
                }
            },
        }
    }

    fn draw_filter_modal(&self, frame: &mut ratatui::Frame, area: Rect) {
        let popup_area = centered_rect(70, 70, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default().borders(Borders::ALL).title("Filter");
        frame.render_widget(block, popup_area);

        let inner = Rect {
            x: popup_area.x + 1,
            y: popup_area.y + 1,
            width: popup_area.width.saturating_sub(2),
            height: popup_area.height.saturating_sub(2),
        };

        let mut lines = Vec::new();
        lines.push(line_field(
            self.filter.active_field() == FilterField::Preset,
            format!("Preset: {}", self.filter.preset.label()),
        ));
        lines.push(line_field(
            self.filter.active_field() == FilterField::Since,
            format!("Since: {}", self.filter.since),
        ));
        lines.push(line_field(
            self.filter.active_field() == FilterField::Until,
            format!("Until: {}", self.filter.until),
        ));
        lines.push(line_field(
            self.filter.active_field() == FilterField::GroupBy,
            format!("Group-by: {:?}", self.filter.group_by),
        ));

        match self.tool {
            ToolKind::Codex => {
                lines.push(line_field(
                    self.filter.active_field() == FilterField::Breakdown,
                    format!(
                        "Breakdown: {}",
                        if self.filter.with_breakdown {
                            "on"
                        } else {
                            "off"
                        }
                    ),
                ));
            }
            ToolKind::ClaudeCode => {
                lines.push(line_field(
                    self.filter.active_field() == FilterField::Sidechain,
                    format!(
                        "Include sidechain: {}",
                        if self.filter.include_sidechain {
                            "yes"
                        } else {
                            "no"
                        }
                    ),
                ));
            }
            ToolKind::Cursor => {}
        }

        lines.push(Line::from(""));
        lines.push(line_field(
            self.filter.active_field() == FilterField::Apply,
            "Apply (Enter)".to_string(),
        ));
        lines.push(line_field(
            self.filter.active_field() == FilterField::Cancel,
            "Cancel (Enter/Esc)".to_string(),
        ));

        if let Some(err) = &self.filter.error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Error: {err}"),
                Style::default().fg(Color::Red),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .block(Block::default())
            .wrap(Wrap { trim: true });
        frame.render_widget(paragraph, inner);
    }
}

fn tool_title(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Codex => "codex stats",
        ToolKind::ClaudeCode => "claude-code stats",
        ToolKind::Cursor => "cursor stats",
    }
}

fn line_field(active: bool, text: String) -> Line<'static> {
    if active {
        Line::from(Span::styled(
            format!("> {text}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(Span::raw(format!("  {text}")))
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_form_last_7d_builds_last_args() {
        let request = StatsTuiScanRequest {
            cwd: "/p".into(),
            group_by: GroupBy::Day,
            range: TimeRangeArgs::default(),
            limit: 200,
            verbose_paths: false,
            with_breakdown: false,
            include_sidechain: true,
        };

        let mut form = FilterFormState::new(ToolKind::Codex, &request);
        form.preset = TimePreset::Last7d;
        let range = form.build_range_args();
        assert_eq!(range.last.as_deref(), Some("7d"));
        assert!(range.since.is_none());
        assert!(range.until.is_none());
    }

    #[test]
    fn filter_form_custom_builds_since_until() {
        let request = StatsTuiScanRequest {
            cwd: "/p".into(),
            group_by: GroupBy::Day,
            range: TimeRangeArgs::default(),
            limit: 200,
            verbose_paths: false,
            with_breakdown: false,
            include_sidechain: true,
        };

        let mut form = FilterFormState::new(ToolKind::Codex, &request);
        form.preset = TimePreset::Custom;
        form.since = "2026-02-01".to_string();
        form.until = "2026-03-01".to_string();
        let range = form.build_range_args();
        assert_eq!(range.since.as_deref(), Some("2026-02-01"));
        assert_eq!(range.until.as_deref(), Some("2026-03-01"));
        assert!(range.last.is_none());
    }
}

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
use ratatui::widgets::{
    Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Sparkline, Table, TableState, Tabs, Wrap,
};
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
    summary: crate::usage_stats::SummaryView,
    trend: crate::usage_stats::TrendView,
}

#[derive(Clone, Debug)]
enum ScanStatus {
    Idle,
    Scanning(Option<ScanProgress>),
    Ready(Box<ScanResultCache>),
    Error(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrendMetric {
    Overall,
    Primary,
    Sidechain,
}

impl TrendMetric {
    fn label(self) -> &'static str {
        match self {
            Self::Overall => "overall",
            Self::Primary => "primary",
            Self::Sidechain => "sidechain",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Overall => Self::Primary,
            Self::Primary => Self::Sidechain,
            Self::Sidechain => Self::Overall,
        }
    }
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
    sessions_table_state: TableState,
    trend_metric: TrendMetric,
    trend_table_state: TableState,
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
            sessions_table_state: TableState::default(),
            trend_metric: TrendMetric::Overall,
            trend_table_state: TableState::default(),
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
                    let cache = self.build_cache(&sessions);
                    self.sessions_table_state
                        .select((!cache.sessions_list.is_empty()).then_some(0));
                    self.trend_table_state.select(
                        (!cache.trend.buckets.is_empty())
                            .then_some(cache.trend.buckets.len().saturating_sub(1)),
                    );
                    self.status = ScanStatus::Ready(Box::new(cache));
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

        ScanResultCache {
            sessions_all: sessions.to_vec(),
            sessions_list: sessions_view.sessions,
            summary,
            trend,
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
            KeyCode::Up | KeyCode::Char('k') => match self.tab {
                StatsTab::Sessions => self.sessions_move(-1),
                StatsTab::Trend => self.trend_move(-1),
                _ => {}
            },
            KeyCode::Down | KeyCode::Char('j') => match self.tab {
                StatsTab::Sessions => self.sessions_move(1),
                StatsTab::Trend => self.trend_move(1),
                _ => {}
            },
            KeyCode::Enter => self.sessions_enter(),
            KeyCode::Char('c') => self.trend_cycle_metric(),
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
            self.sessions_table_state.select(None);
            return;
        }
        let current = self.sessions_table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, cache.sessions_list.len().saturating_sub(1) as isize);
        self.sessions_table_state.select(Some(next as usize));
    }

    fn trend_move(&mut self, delta: isize) {
        if self.tab != StatsTab::Trend {
            return;
        }
        let ScanStatus::Ready(cache) = &self.status else {
            return;
        };
        if cache.trend.buckets.is_empty() {
            self.trend_table_state.select(None);
            return;
        }
        let current = self.trend_table_state.selected().unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, cache.trend.buckets.len().saturating_sub(1) as isize);
        self.trend_table_state.select(Some(next as usize));
    }

    fn trend_cycle_metric(&mut self) {
        if self.tab != StatsTab::Trend {
            return;
        }
        if self.tool != ToolKind::ClaudeCode {
            return;
        }
        self.trend_metric = self.trend_metric.next();
    }

    fn sessions_enter(&mut self) {
        if self.tab != StatsTab::Sessions {
            return;
        }
        let ScanStatus::Ready(cache) = &self.status else {
            return;
        };
        let Some(idx) = self.sessions_table_state.selected() else {
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
            .constraints(
                [
                    Constraint::Length(3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(area);

        self.draw_tabs(frame, chunks[0]);
        self.draw_body(frame, chunks[1]);
        self.draw_help(frame, chunks[2]);

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
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .select(selected);
        frame.render_widget(tabs, area);
    }

    fn draw_help(&self, frame: &mut ratatui::Frame, area: Rect) {
        let help = "Tab/Shift+Tab switch · ↑↓/j/k move · Enter drilldown · f filter · c cycle(trend/Claude) · q quit";
        frame.render_widget(
            Paragraph::new(Line::from(help)).style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }

    fn draw_overview(
        frame: &mut ratatui::Frame,
        area: Rect,
        summary: &crate::usage_stats::SummaryView,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(area);

        let top = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[0]);
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[1]);

        let mut token_lines = Vec::new();
        token_lines.push(Line::from(Span::styled(
            format_u64_count(summary.totals.tokens_total_known),
            Style::default().add_modifier(Modifier::BOLD),
        )));
        if let Some(sidechain) = &summary.sidechain_totals {
            token_lines.push(Line::from(format!(
                "primary {} · side {}",
                format_u64_count(sidechain.primary.tokens_total_known),
                format_u64_count(sidechain.sidechain.tokens_total_known)
            )));
        }
        frame.render_widget(
            Paragraph::new(token_lines)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Tokens (known-only)"),
                )
                .wrap(Wrap { trim: true }),
            top[0],
        );

        let sessions_total = summary.coverage.total_sessions;
        let sessions_known = summary.coverage.known_token_sessions;
        let sessions_lines = vec![
            Line::from(Span::styled(
                format_usize_count(sessions_total),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(format!(
                "known tokens: {}",
                format_usize_count(sessions_known)
            )),
        ];
        frame.render_widget(
            Paragraph::new(sessions_lines)
                .block(Block::default().borders(Borders::ALL).title("Sessions"))
                .wrap(Wrap { trim: true }),
            top[1],
        );

        let coverage_percent = (sessions_known.saturating_mul(100))
            .checked_div(sessions_total)
            .unwrap_or(0)
            .min(100) as u16;
        let coverage_label = format!(
            "{}/{} ({}%)",
            format_usize_count(sessions_known),
            format_usize_count(sessions_total),
            coverage_percent
        );
        frame.render_widget(
            Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Coverage"))
                .gauge_style(Style::default().fg(Color::Green))
                .label(coverage_label)
                .percent(coverage_percent),
            bottom[0],
        );

        let latest = summary
            .latest_end_ts
            .map(|ts| {
                ts.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_else(|| "-".to_string());
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                latest,
                Style::default().add_modifier(Modifier::BOLD),
            )))
            .block(Block::default().borders(Borders::ALL).title("Latest"))
            .wrap(Wrap { trim: true }),
            bottom[1],
        );
    }

    fn draw_trend(
        frame: &mut ratatui::Frame,
        area: Rect,
        trend: &crate::usage_stats::TrendView,
        tool: ToolKind,
        metric: TrendMetric,
        table_state: &mut TableState,
    ) {
        let values = trend
            .buckets
            .iter()
            .map(|bucket| bucket_tokens(bucket, metric).unwrap_or(0))
            .collect::<Vec<_>>();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(7), Constraint::Min(1)].as_ref())
            .split(area);

        let title = if tool == ToolKind::ClaudeCode {
            format!("Tokens ({})", metric.label())
        } else {
            "Tokens".to_string()
        };
        frame.render_widget(
            Sparkline::default()
                .block(Block::default().borders(Borders::ALL).title(title))
                .data(values)
                .style(Style::default().fg(Color::Cyan)),
            chunks[0],
        );

        let header = Row::new(["Bucket", "Tokens", "Sessions (known/total)"]).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
        let rows = trend
            .buckets
            .iter()
            .map(|bucket| {
                let tokens = bucket_tokens(bucket, metric)
                    .map(format_u64_count)
                    .unwrap_or_else(|| "-".to_string());
                Row::new(vec![
                    Cell::new(bucket.label.clone()),
                    Cell::new(tokens),
                    Cell::new(format!(
                        "{}/{}",
                        format_usize_count(bucket.coverage.known_token_sessions),
                        format_usize_count(bucket.coverage.total_sessions)
                    )),
                ])
            })
            .collect::<Vec<_>>();

        let table = Table::new(
            rows,
            [
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Min(20),
            ],
        )
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Buckets"))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("> ");

        frame.render_stateful_widget(table, chunks[1], table_state);
    }

    fn draw_body(&mut self, frame: &mut ratatui::Frame, area: Rect) {
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
                    Self::draw_overview(frame, area, &cache.summary);
                }
                StatsTab::Trend => {
                    let metric = if self.tool == ToolKind::ClaudeCode {
                        self.trend_metric
                    } else {
                        TrendMetric::Overall
                    };
                    Self::draw_trend(
                        frame,
                        area,
                        &cache.trend,
                        self.tool,
                        metric,
                        &mut self.trend_table_state,
                    );
                }
                StatsTab::Sessions => {
                    let is_claude = self.tool == ToolKind::ClaudeCode;
                    let header = if is_claude {
                        Row::new(["End", "Known", "Id", "SC", "Cwd", "Title"])
                    } else {
                        Row::new(["End", "Known", "Id", "Cwd", "Title"])
                    }
                    .style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    );

                    let rows = cache
                        .sessions_list
                        .iter()
                        .map(|session| {
                            let end = session
                                .end_ts
                                .with_timezone(&chrono::Local)
                                .format("%Y-%m-%d %H:%M")
                                .to_string();
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

                            if is_claude {
                                let sidechain = match session.is_sidechain {
                                    Some(true) => "S",
                                    Some(false) => "",
                                    None => "-",
                                };
                                Row::new(vec![
                                    Cell::new(end),
                                    Cell::new(tokens),
                                    Cell::new(session.id.to_string()),
                                    Cell::new(sidechain),
                                    Cell::new(cwd),
                                    Cell::new(title.to_string()),
                                ])
                            } else {
                                Row::new(vec![
                                    Cell::new(end),
                                    Cell::new(tokens),
                                    Cell::new(session.id.to_string()),
                                    Cell::new(cwd),
                                    Cell::new(title.to_string()),
                                ])
                            }
                        })
                        .collect::<Vec<_>>();

                    let widths = if is_claude {
                        vec![
                            Constraint::Length(16),
                            Constraint::Length(10),
                            Constraint::Length(24),
                            Constraint::Length(2),
                            Constraint::Min(24),
                            Constraint::Min(20),
                        ]
                    } else {
                        vec![
                            Constraint::Length(16),
                            Constraint::Length(10),
                            Constraint::Length(24),
                            Constraint::Min(24),
                            Constraint::Min(20),
                        ]
                    };

                    let table = Table::new(rows, widths)
                        .header(header)
                        .block(Block::default().borders(Borders::ALL).title("Sessions"))
                        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                        .highlight_symbol("> ");

                    frame.render_stateful_widget(table, area, &mut self.sessions_table_state);
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

fn format_u64_count(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len().saturating_add(s.len() / 3));
    for (idx, ch) in s.chars().enumerate() {
        out.push(ch);
        let remaining = s.len().saturating_sub(idx).saturating_sub(1);
        if remaining > 0 && remaining % 3 == 0 {
            out.push(',');
        }
    }
    out
}

fn format_usize_count(value: usize) -> String {
    format_u64_count(value as u64)
}

fn bucket_tokens(bucket: &crate::usage_stats::StatsBucket, metric: TrendMetric) -> Option<u64> {
    match metric {
        TrendMetric::Overall => Some(bucket.totals.tokens_total_known),
        TrendMetric::Primary => bucket
            .sidechain_totals
            .as_ref()
            .map(|totals| totals.primary.tokens_total_known),
        TrendMetric::Sidechain => bucket
            .sidechain_totals
            .as_ref()
            .map(|totals| totals.sidechain.tokens_total_known),
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
    use chrono::TimeZone;
    use std::path::PathBuf;

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

    fn make_app(tool: ToolKind) -> StatsTuiApp {
        let request = StatsTuiScanRequest {
            cwd: PathBuf::from("/p"),
            group_by: GroupBy::Day,
            range: TimeRangeArgs::default(),
            limit: 200,
            verbose_paths: false,
            with_breakdown: false,
            include_sidechain: true,
        };
        let scan_fn: ScanFn = Arc::new(|_request, _tx| Ok(vec![]));
        StatsTuiApp::new(tool, request, scan_fn)
    }

    fn make_session(id: &str, end_ts: chrono::DateTime<chrono::Utc>) -> SessionRecord {
        SessionRecord {
            tool: ToolKind::Codex,
            id: SessionId(id.to_string()),
            cwd: PathBuf::from("/p"),
            title: None,
            start_ts: None,
            end_ts,
            token_usage: crate::usage_stats::TokenUsage {
                total: Some(1),
                ..crate::usage_stats::TokenUsage::default()
            },
            is_sidechain: None,
        }
    }

    #[test]
    fn trend_metric_cycles_for_claude_in_trend_tab() {
        let mut app = make_app(ToolKind::ClaudeCode);
        app.tab = StatsTab::Trend;
        assert_eq!(app.trend_metric, TrendMetric::Overall);
        app.trend_cycle_metric();
        assert_eq!(app.trend_metric, TrendMetric::Primary);
        app.trend_cycle_metric();
        assert_eq!(app.trend_metric, TrendMetric::Sidechain);
        app.trend_cycle_metric();
        assert_eq!(app.trend_metric, TrendMetric::Overall);
    }

    #[test]
    fn trend_move_updates_table_selection() {
        let mut app = make_app(ToolKind::Codex);
        app.tab = StatsTab::Trend;

        let sessions = vec![
            make_session(
                "t1",
                chrono::Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            ),
            make_session(
                "t2",
                chrono::Utc.with_ymd_and_hms(2026, 1, 10, 12, 0, 0).unwrap(),
            ),
        ];
        let cache = app.build_cache(&sessions);
        app.status = ScanStatus::Ready(Box::new(cache));
        app.trend_table_state.select(Some(0));

        app.trend_move(1);
        assert_eq!(app.trend_table_state.selected(), Some(1));
        app.trend_move(1);
        assert_eq!(app.trend_table_state.selected(), Some(1));
        app.trend_move(-1);
        assert_eq!(app.trend_table_state.selected(), Some(0));
    }

    #[test]
    fn sessions_move_updates_table_selection() {
        let mut app = make_app(ToolKind::Codex);
        app.tab = StatsTab::Sessions;

        let sessions = vec![
            make_session(
                "t1",
                chrono::Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap(),
            ),
            make_session(
                "t2",
                chrono::Utc.with_ymd_and_hms(2026, 1, 2, 12, 0, 0).unwrap(),
            ),
        ];
        let cache = app.build_cache(&sessions);
        app.status = ScanStatus::Ready(Box::new(cache));
        app.sessions_table_state.select(Some(0));

        app.sessions_move(1);
        assert_eq!(app.sessions_table_state.selected(), Some(1));
        app.sessions_move(1);
        assert_eq!(app.sessions_table_state.selected(), Some(1));
        app.sessions_move(-1);
        assert_eq!(app.sessions_table_state.selected(), Some(0));
    }
}

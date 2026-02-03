#![forbid(unsafe_code)]

//! Live Log Search & Filter demo screen.
//!
//! Demonstrates the [`LogViewer`] widget with real-time search, filtering,
//! and streaming log lines. Shows:
//!
//! - Streaming log append with auto-scroll (follow mode)
//! - `/` to open inline search bar
//! - `n` / `N` for next/prev match navigation
//! - `f` to toggle filter mode (show only matching lines)
//! - Case sensitivity toggle (Ctrl+C in search mode)
//! - Context lines toggle (Ctrl+X in search mode)
//! - Match count and current position indicator

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui_core::geometry::Rect;
use ftui_layout::{Constraint, Flex};
use ftui_render::frame::Frame;
use ftui_runtime::Cmd;
use ftui_style::Style;
use ftui_text::Text;
use ftui_widgets::block::Block;
use ftui_widgets::borders::{BorderType, Borders};
use ftui_widgets::log_viewer::{LogViewer, LogViewerState, LogWrapMode, SearchConfig, SearchMode};
use ftui_widgets::paragraph::Paragraph;
use ftui_widgets::{StatefulWidget, Widget};

use super::{HelpEntry, Screen};
use crate::theme;

/// Interval between simulated log line bursts (in ticks).
const LOG_BURST_INTERVAL: u64 = 3;
/// Lines per burst.
const LOG_BURST_SIZE: usize = 2;
/// Max lines retained in the viewer.
const MAX_LOG_LINES: usize = 5_000;

/// UI mode for the search bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UiMode {
    /// Normal log viewing mode.
    Normal,
    /// Search bar is open and accepting input.
    Search,
    /// Filter bar is open and accepting input.
    Filter,
}

/// Log Search demo screen state.
pub struct LogSearch {
    viewer: LogViewer,
    viewer_state: LogViewerState,
    mode: UiMode,
    query: String,
    last_search: String,
    search_config: SearchConfig,
    filter_active: bool,
    filter_query: String,
    tick_count: u64,
    lines_generated: u64,
    paused: bool,
}

impl Default for LogSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSearch {
    pub fn new() -> Self {
        let mut viewer = LogViewer::new(MAX_LOG_LINES)
            .wrap_mode(LogWrapMode::NoWrap)
            .search_highlight_style(
                Style::new()
                    .fg(theme::bg::BASE)
                    .bg(theme::accent::WARNING)
                    .bold(),
            );

        for i in 0..50 {
            viewer.push(generate_log_line(i));
        }

        Self {
            viewer,
            viewer_state: LogViewerState::default(),
            mode: UiMode::Normal,
            query: String::new(),
            last_search: String::new(),
            search_config: SearchConfig {
                mode: SearchMode::Literal,
                case_sensitive: false,
                context_lines: 0,
            },
            filter_active: false,
            filter_query: String::new(),
            tick_count: 0,
            lines_generated: 50,
            paused: false,
        }
    }

    fn submit_search(&mut self) {
        if self.query.is_empty() {
            self.viewer.clear_search();
            self.last_search.clear();
        } else {
            self.last_search = self.query.clone();
            self.viewer
                .search_with_config(&self.query, self.search_config.clone());
        }
        self.mode = UiMode::Normal;
    }

    fn submit_filter(&mut self) {
        if self.query.is_empty() {
            self.viewer.set_filter(None);
            self.filter_active = false;
            self.filter_query.clear();
        } else {
            self.filter_query = self.query.clone();
            self.viewer.set_filter(Some(&self.query));
            self.filter_active = true;
        }
        self.mode = UiMode::Normal;
    }

    fn handle_normal_key(&mut self, key: &KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Char('/'), Modifiers::NONE) => {
                self.mode = UiMode::Search;
                self.query = self.last_search.clone();
            }
            (KeyCode::Char('f'), Modifiers::NONE) => {
                self.mode = UiMode::Filter;
                self.query = self.filter_query.clone();
            }
            (KeyCode::Char('n'), Modifiers::NONE) => {
                if !self.last_search.is_empty() {
                    self.viewer.next_match();
                }
            }
            (KeyCode::Char('N'), Modifiers::NONE) => {
                if !self.last_search.is_empty() {
                    self.viewer.prev_match();
                }
            }
            (KeyCode::Char('F'), Modifiers::NONE) => {
                self.viewer.set_filter(None);
                self.filter_active = false;
                self.filter_query.clear();
            }
            (KeyCode::Char(' '), Modifiers::NONE) => {
                self.paused = !self.paused;
            }
            (KeyCode::Char('g'), Modifiers::NONE) => {
                self.viewer.scroll_to_top();
            }
            (KeyCode::Char('G'), Modifiers::NONE) => {
                self.viewer.scroll_to_bottom();
            }
            (KeyCode::Up, _) | (KeyCode::Char('k'), Modifiers::NONE) => {
                self.viewer.scroll_up(1);
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), Modifiers::NONE) => {
                self.viewer.scroll_down(1);
            }
            (KeyCode::PageUp, _) => {
                self.viewer.page_up(&self.viewer_state);
            }
            (KeyCode::PageDown, _) => {
                self.viewer.page_down(&self.viewer_state);
            }
            (KeyCode::Escape, _) => {
                self.viewer.clear_search();
                self.last_search.clear();
            }
            _ => {}
        }
    }

    fn handle_input_key(&mut self, key: &KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Escape, _) => {
                self.query.clear();
                self.mode = UiMode::Normal;
            }
            (KeyCode::Enter, _) => match self.mode {
                UiMode::Search => self.submit_search(),
                UiMode::Filter => self.submit_filter(),
                UiMode::Normal => {}
            },
            (KeyCode::Backspace, _) => {
                self.query.pop();
                self.live_update();
            }
            (KeyCode::Char('u'), m) if m.contains(Modifiers::CTRL) => {
                self.query.clear();
                self.live_update();
            }
            (KeyCode::Char('c'), m) if m.contains(Modifiers::CTRL) => {
                if self.mode == UiMode::Search {
                    self.search_config.case_sensitive = !self.search_config.case_sensitive;
                    self.live_update();
                }
            }
            (KeyCode::Char('r'), m) if m.contains(Modifiers::CTRL) => {
                if self.mode == UiMode::Search {
                    self.search_config.mode = match self.search_config.mode {
                        SearchMode::Literal => SearchMode::Regex,
                        SearchMode::Regex => SearchMode::Literal,
                    };
                    self.live_update();
                }
            }
            (KeyCode::Char('x'), m) if m.contains(Modifiers::CTRL) => {
                if self.mode == UiMode::Search {
                    self.search_config.context_lines = match self.search_config.context_lines {
                        0 => 1,
                        1 => 2,
                        2 => 5,
                        _ => 0,
                    };
                    self.live_update();
                }
            }
            (KeyCode::Char(ch), _) => {
                self.query.push(ch);
                self.live_update();
            }
            _ => {}
        }
    }

    fn live_update(&mut self) {
        match self.mode {
            UiMode::Search => {
                if self.query.is_empty() {
                    self.viewer.clear_search();
                } else {
                    self.viewer
                        .search_with_config(&self.query, self.search_config.clone());
                }
            }
            UiMode::Filter => {
                if self.query.is_empty() {
                    self.viewer.set_filter(None);
                } else {
                    self.viewer.set_filter(Some(&self.query));
                }
            }
            UiMode::Normal => {}
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        if area.height == 0 || area.width < 10 {
            return;
        }

        let mut segments: Vec<String> = Vec::new();

        match self.mode {
            UiMode::Normal => segments.push("NORMAL".into()),
            UiMode::Search => segments.push("SEARCH".into()),
            UiMode::Filter => segments.push("FILTER".into()),
        }

        if self.paused {
            segments.push("PAUSED".into());
        }

        if let Some((current, total)) = self.viewer.search_info() {
            segments.push(format!("{current}/{total}"));
        }

        if self.filter_active {
            segments.push("FILTERED".into());
        }

        if self.mode == UiMode::Search {
            if self.search_config.case_sensitive {
                segments.push("Aa".into());
            } else {
                segments.push("aa".into());
            }
            match self.search_config.mode {
                SearchMode::Literal => segments.push("lit".into()),
                SearchMode::Regex => segments.push("re".into()),
            }
            if self.search_config.context_lines > 0 {
                segments.push(format!("ctx:{}", self.search_config.context_lines));
            }
        }

        let status_text = format!(
            " {} | lines: {} | gen: {} ",
            segments.join(" | "),
            self.viewer.line_count(),
            self.lines_generated,
        );

        let style = Style::new().fg(theme::fg::SECONDARY).bg(theme::bg::SURFACE);
        let para = Paragraph::new(Text::from(status_text)).style(style);
        Widget::render(&para, area, frame);
    }

    fn render_input_bar(&self, frame: &mut Frame, area: Rect) {
        if area.height == 0 {
            return;
        }

        let prefix = match self.mode {
            UiMode::Search => "/",
            UiMode::Filter => "filter: ",
            UiMode::Normal => return,
        };

        let display = format!("{}{}_", prefix, self.query);
        let input_style = Style::new().fg(theme::fg::PRIMARY).bg(theme::bg::OVERLAY);
        let para = Paragraph::new(Text::from(display)).style(input_style);
        Widget::render(&para, area, frame);
    }
}

impl Screen for LogSearch {
    type Message = ();

    fn update(&mut self, event: &Event) -> Cmd<Self::Message> {
        if let Event::Key(key) = event
            && key.kind == KeyEventKind::Press
        {
            match self.mode {
                UiMode::Normal => self.handle_normal_key(key),
                UiMode::Search | UiMode::Filter => self.handle_input_key(key),
            }
        }
        Cmd::none()
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.width < 4 || area.height < 4 {
            return;
        }

        let input_active = self.mode != UiMode::Normal;
        let bar_height = if input_active { 2 } else { 1 };

        let sections = Flex::vertical()
            .constraints([Constraint::Min(3), Constraint::Fixed(bar_height)])
            .split(area);

        let log_area = sections[0];
        let bar_area = sections[1];

        let title = if self.filter_active {
            format!(" Log Viewer [filter: {}] ", self.filter_query)
        } else {
            " Log Viewer ".to_string()
        };

        let border_style = if self.mode == UiMode::Normal {
            Style::new().fg(theme::fg::MUTED)
        } else {
            Style::new().fg(theme::accent::WARNING)
        };

        let block = Block::new()
            .title(&title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(log_area);
        Widget::render(&block, log_area, frame);

        let mut state = self.viewer_state.clone();
        StatefulWidget::render(&self.viewer, inner, frame, &mut state);

        if input_active {
            let bar_sections = Flex::vertical()
                .constraints([Constraint::Fixed(1), Constraint::Fixed(1)])
                .split(bar_area);
            self.render_input_bar(frame, bar_sections[0]);
            self.render_status_bar(frame, bar_sections[1]);
        } else {
            self.render_status_bar(frame, bar_area);
        }
    }

    fn tick(&mut self, tick_count: u64) {
        self.tick_count = tick_count;

        if !self.paused && tick_count.is_multiple_of(LOG_BURST_INTERVAL) {
            for _ in 0..LOG_BURST_SIZE {
                self.viewer.push(generate_log_line(self.lines_generated));
                self.lines_generated += 1;
            }
        }
    }

    fn keybindings(&self) -> Vec<HelpEntry> {
        vec![
            HelpEntry {
                key: "/",
                action: "Open search bar",
            },
            HelpEntry {
                key: "f",
                action: "Open filter bar",
            },
            HelpEntry {
                key: "n / N",
                action: "Next / previous match",
            },
            HelpEntry {
                key: "F",
                action: "Clear filter",
            },
            HelpEntry {
                key: "Esc",
                action: "Close search / clear highlights",
            },
            HelpEntry {
                key: "Space",
                action: "Pause / resume log stream",
            },
            HelpEntry {
                key: "g / G",
                action: "Go to top / bottom",
            },
            HelpEntry {
                key: "j/k",
                action: "Scroll up / down",
            },
            HelpEntry {
                key: "Ctrl+C",
                action: "Toggle case sensitivity (search)",
            },
            HelpEntry {
                key: "Ctrl+R",
                action: "Toggle regex mode (search)",
            },
            HelpEntry {
                key: "Ctrl+X",
                action: "Cycle context lines (search)",
            },
        ]
    }

    fn title(&self) -> &'static str {
        "Log Search"
    }

    fn tab_label(&self) -> &'static str {
        "Logs"
    }
}

fn generate_log_line(seq: u64) -> Text {
    let (severity_label, severity_color) = match seq % 13 {
        0..=5 => ("INFO", theme::accent::INFO),
        6..=8 => ("DEBUG", theme::fg::MUTED),
        9..=10 => ("WARN", theme::accent::WARNING),
        11 => ("ERROR", theme::accent::ERROR),
        _ => ("TRACE", theme::fg::MUTED),
    };

    let module = match seq % 9 {
        0 => "server::http",
        1 => "db::pool",
        2 => "auth::jwt",
        3 => "cache::redis",
        4 => "queue::worker",
        5 => "api::handler",
        6 => "core::runtime",
        7 => "metrics::push",
        _ => "config::reload",
    };

    let message = match seq % 11 {
        0 => "Request processed successfully",
        1 => "Connection pool health check passed",
        2 => "Token refresh completed for session",
        3 => "Cache hit ratio: 0.94",
        4 => "Worker picked up job from queue",
        5 => "Rate limit threshold approaching",
        6 => "Garbage collection cycle completed",
        7 => "Metric batch flushed to backend",
        8 => "Configuration hot-reload triggered",
        9 => "Retry attempt 2/3 for downstream call",
        _ => "Scheduled maintenance window check",
    };

    let line = format!(
        "[{:>6}] {:>5} {:<18} {}",
        seq, severity_label, module, message
    );

    Text::styled(line, Style::new().fg(severity_color))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_press(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: Modifiers::NONE,
            kind: KeyEventKind::Press,
        })
    }

    fn type_chars(screen: &mut LogSearch, s: &str) {
        for ch in s.chars() {
            screen.update(&key_press(KeyCode::Char(ch)));
        }
    }

    #[test]
    fn test_new_creates_initial_lines() {
        let screen = LogSearch::new();
        assert_eq!(screen.viewer.line_count(), 50);
        assert_eq!(screen.mode, UiMode::Normal);
    }

    #[test]
    fn test_search_mode_toggle() {
        let mut screen = LogSearch::new();
        assert_eq!(screen.mode, UiMode::Normal);

        screen.update(&key_press(KeyCode::Char('/')));
        assert_eq!(screen.mode, UiMode::Search);

        screen.update(&key_press(KeyCode::Escape));
        assert_eq!(screen.mode, UiMode::Normal);
    }

    #[test]
    fn test_filter_mode_toggle() {
        let mut screen = LogSearch::new();
        screen.update(&key_press(KeyCode::Char('f')));
        assert_eq!(screen.mode, UiMode::Filter);
    }

    #[test]
    fn test_search_and_navigate() {
        let mut screen = LogSearch::new();

        screen.update(&key_press(KeyCode::Char('/')));
        type_chars(&mut screen, "ERROR");
        screen.update(&key_press(KeyCode::Enter));

        assert_eq!(screen.mode, UiMode::Normal);
        assert_eq!(screen.last_search, "ERROR");
        assert!(screen.viewer.search_info().is_some());

        let initial_info = screen.viewer.search_info();
        screen.update(&key_press(KeyCode::Char('n')));
        if let Some((_, total)) = initial_info && total > 1 {
            let (current, _) = screen.viewer.search_info().unwrap();
            assert_eq!(current, 2);
        }
    }

    #[test]
    fn test_tick_generates_lines() {
        let mut screen = LogSearch::new();
        let initial = screen.viewer.line_count();
        screen.tick(LOG_BURST_INTERVAL);
        assert_eq!(screen.viewer.line_count(), initial + LOG_BURST_SIZE);
    }

    #[test]
    fn test_pause_stops_generation() {
        let mut screen = LogSearch::new();
        let initial = screen.viewer.line_count();
        screen.update(&key_press(KeyCode::Char(' ')));
        assert!(screen.paused);
        screen.tick(LOG_BURST_INTERVAL);
        assert_eq!(screen.viewer.line_count(), initial);
    }

    #[test]
    fn test_filter_submit() {
        let mut screen = LogSearch::new();
        screen.update(&key_press(KeyCode::Char('f')));
        type_chars(&mut screen, "ERROR");
        screen.update(&key_press(KeyCode::Enter));
        assert!(screen.filter_active);
        assert_eq!(screen.filter_query, "ERROR");
    }

    #[test]
    fn test_generate_log_line_deterministic() {
        let a = generate_log_line(42).to_plain_text();
        let b = generate_log_line(42).to_plain_text();
        assert_eq!(a, b);
    }

    #[test]
    fn test_keybindings_listed() {
        let screen = LogSearch::new();
        let bindings = screen.keybindings();
        assert!(bindings.len() >= 8);
        assert!(bindings.iter().any(|h| h.key == "/"));
        assert!(bindings.iter().any(|h| h.key == "n / N"));
    }
}

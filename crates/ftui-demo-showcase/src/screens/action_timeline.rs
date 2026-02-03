#![forbid(unsafe_code)]

//! Action Timeline / Event Stream Viewer screen.
//!
//! Shows a live event timeline with filters and a detail panel. The timeline
//! is deterministic and uses a bounded ring buffer to keep allocations stable.

use std::collections::VecDeque;

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind, Modifiers};
use ftui_core::geometry::Rect;
use ftui_layout::{Constraint, Flex};
use ftui_render::frame::Frame;
use ftui_runtime::Cmd;
use ftui_style::Style;
use ftui_widgets::Widget;
use ftui_widgets::block::{Alignment, Block};
use ftui_widgets::borders::{BorderType, Borders};
use ftui_widgets::paragraph::Paragraph;

use super::{HelpEntry, Screen};
use crate::theme;

const MAX_EVENTS: usize = 500;
const EVENT_BURST_EVERY: u64 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Severity {
    const ALL: [Severity; 5] = [
        Severity::Trace,
        Severity::Debug,
        Severity::Info,
        Severity::Warn,
        Severity::Error,
    ];

    fn label(self) -> &'static str {
        match self {
            Severity::Trace => "TRACE",
            Severity::Debug => "DEBUG",
            Severity::Info => "INFO",
            Severity::Warn => "WARN",
            Severity::Error => "ERROR",
        }
    }

    fn color(self) -> theme::ColorToken {
        match self {
            Severity::Trace => theme::fg::DISABLED,
            Severity::Debug => theme::fg::MUTED,
            Severity::Info => theme::fg::PRIMARY,
            Severity::Warn => theme::accent::WARNING,
            Severity::Error => theme::accent::ERROR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Core,
    Runtime,
    Render,
    Widgets,
}

impl Component {
    const ALL: [Component; 4] = [
        Component::Core,
        Component::Runtime,
        Component::Render,
        Component::Widgets,
    ];

    fn label(self) -> &'static str {
        match self {
            Component::Core => "core",
            Component::Runtime => "runtime",
            Component::Render => "render",
            Component::Widgets => "widgets",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventKind {
    Input,
    Command,
    Subscription,
    Render,
    Present,
    Capability,
    Degrade,
}

impl EventKind {
    const ALL: [EventKind; 7] = [
        EventKind::Input,
        EventKind::Command,
        EventKind::Subscription,
        EventKind::Render,
        EventKind::Present,
        EventKind::Capability,
        EventKind::Degrade,
    ];

    fn label(self) -> &'static str {
        match self {
            EventKind::Input => "input",
            EventKind::Command => "cmd",
            EventKind::Subscription => "sub",
            EventKind::Render => "render",
            EventKind::Present => "present",
            EventKind::Capability => "caps",
            EventKind::Degrade => "budget",
        }
    }
}

#[derive(Debug, Clone)]
struct TimelineEvent {
    id: u64,
    tick: u64,
    severity: Severity,
    component: Component,
    kind: EventKind,
    summary: String,
    fields: Vec<(String, String)>,
    evidence: Option<String>,
}

pub struct ActionTimeline {
    events: VecDeque<TimelineEvent>,
    selected: usize,
    scroll_offset: usize,
    viewport_height: usize,
    follow: bool,
    show_details: bool,
    filter_component: Option<Component>,
    filter_severity: Option<Severity>,
    filter_kind: Option<EventKind>,
    next_id: u64,
    tick_count: u64,
}

impl Default for ActionTimeline {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionTimeline {
    pub fn new() -> Self {
        let mut timeline = Self {
            events: VecDeque::with_capacity(MAX_EVENTS),
            selected: 0,
            scroll_offset: 0,
            viewport_height: 12,
            follow: true,
            show_details: true,
            filter_component: None,
            filter_severity: None,
            filter_kind: None,
            next_id: 1,
            tick_count: 0,
        };
        for tick in 0..12 {
            timeline.tick_count = tick;
            timeline.push_event(timeline.synthetic_event());
        }
        timeline
    }

    fn push_event(&mut self, event: TimelineEvent) {
        if self.events.len() == MAX_EVENTS {
            self.events.pop_front();
            if self.selected > 0 {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        self.events.push_back(event);
    }

    fn synthetic_event(&mut self) -> TimelineEvent {
        let tick = self.tick_count;
        let severity = Severity::ALL[(tick as usize) % Severity::ALL.len()];
        let component = Component::ALL[(tick as usize / 2) % Component::ALL.len()];
        let kind = EventKind::ALL[(tick as usize / 3) % EventKind::ALL.len()];
        let id = self.next_id;
        self.next_id += 1;

        let summary = match kind {
            EventKind::Input => "Key event processed".to_string(),
            EventKind::Command => "Command dispatched to model".to_string(),
            EventKind::Subscription => "Subscription tick delivered".to_string(),
            EventKind::Render => "Frame diff computed".to_string(),
            EventKind::Present => "Presenter emitted ANSI batch".to_string(),
            EventKind::Capability => "Capability probe updated".to_string(),
            EventKind::Degrade => "Render budget degraded".to_string(),
        };

        let latency_ms = 2 + (tick % 7) * 3;
        let fields = vec![
            ("latency_ms".to_string(), latency_ms.to_string()),
            ("diff_cells".to_string(), ((tick * 13) % 120).to_string()),
            ("ansi_bytes".to_string(), ((tick * 47) % 2048).to_string()),
        ];

        let evidence = match kind {
            EventKind::Capability => Some("evidence: env + probe signal".to_string()),
            EventKind::Degrade => Some("budget: frame_time > p95".to_string()),
            _ => None,
        };

        TimelineEvent {
            id,
            tick,
            severity,
            component,
            kind,
            summary,
            fields,
            evidence,
        }
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        for (idx, event) in self.events.iter().enumerate() {
            if self
                .filter_component
                .map_or(true, |c| c == event.component)
                && self.filter_severity.map_or(true, |s| s == event.severity)
                && self.filter_kind.map_or(true, |k| k == event.kind)
            {
                indices.push(idx);
            }
        }
        indices
    }

    fn ensure_selection(&mut self, filtered_len: usize) {
        if filtered_len == 0 {
            self.selected = 0;
            self.scroll_offset = 0;
            return;
        }

        if self.follow {
            self.selected = filtered_len - 1;
        } else if self.selected >= filtered_len {
            self.selected = filtered_len - 1;
        }

        self.ensure_visible(filtered_len);
    }

    fn ensure_visible(&mut self, filtered_len: usize) {
        if filtered_len == 0 {
            self.scroll_offset = 0;
            return;
        }
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        }
        if self.selected >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = self.selected.saturating_sub(self.viewport_height - 1);
        }
    }

    fn cycle_component(&mut self) {
        self.filter_component = match self.filter_component {
            None => Some(Component::Core),
            Some(Component::Core) => Some(Component::Runtime),
            Some(Component::Runtime) => Some(Component::Render),
            Some(Component::Render) => Some(Component::Widgets),
            Some(Component::Widgets) => None,
        };
    }

    fn cycle_severity(&mut self) {
        self.filter_severity = match self.filter_severity {
            None => Some(Severity::Info),
            Some(Severity::Trace) => Some(Severity::Debug),
            Some(Severity::Debug) => Some(Severity::Info),
            Some(Severity::Info) => Some(Severity::Warn),
            Some(Severity::Warn) => Some(Severity::Error),
            Some(Severity::Error) => None,
        };
    }

    fn cycle_kind(&mut self) {
        self.filter_kind = match self.filter_kind {
            None => Some(EventKind::Input),
            Some(EventKind::Input) => Some(EventKind::Command),
            Some(EventKind::Command) => Some(EventKind::Subscription),
            Some(EventKind::Subscription) => Some(EventKind::Render),
            Some(EventKind::Render) => Some(EventKind::Present),
            Some(EventKind::Present) => Some(EventKind::Capability),
            Some(EventKind::Capability) => Some(EventKind::Degrade),
            Some(EventKind::Degrade) => None,
        };
    }

    fn clear_filters(&mut self) {
        self.filter_component = None;
        self.filter_severity = None;
        self.filter_kind = None;
    }

    fn render_filters(&self, frame: &mut Frame, area: Rect) {
        let border_style = Style::new().fg(theme::screen_accent::ADVANCED);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Filters + Follow")
            .title_alignment(Alignment::Center)
            .style(border_style);
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let component = self
            .filter_component
            .map(|c| c.label())
            .unwrap_or("all");
        let severity = self
            .filter_severity
            .map(|s| s.label())
            .unwrap_or("all");
        let kind = self.filter_kind.map(|k| k.label()).unwrap_or("all");
        let follow = if self.follow { "ON" } else { "OFF" };

        let line = format!(
            "Follow[F]: {follow}  Component[C]: {component}  Severity[S]: {severity}  Type[T]: {kind}  Clear[X]"
        );
        Paragraph::new(line)
            .style(theme::body())
            .render(inner, frame);
    }

    fn render_timeline(&self, frame: &mut Frame, area: Rect, filtered: &[usize]) {
        let border_style = Style::new().fg(theme::screen_accent::ADVANCED);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Event Timeline")
            .title_alignment(Alignment::Center)
            .style(border_style);
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        if filtered.is_empty() {
            Paragraph::new("No events match current filters.")
                .style(theme::muted())
                .render(inner, frame);
            return;
        }

        let viewport_height = inner.height.max(1) as usize;
        let max_index = filtered.len().saturating_sub(1);
        let selected = self.selected.min(max_index);
        let mut scroll_offset = self.scroll_offset.min(max_index);
        if selected < scroll_offset {
            scroll_offset = selected;
        }
        if selected >= scroll_offset + viewport_height {
            scroll_offset = selected.saturating_sub(viewport_height - 1);
        }

        let end = (scroll_offset + viewport_height).min(filtered.len());
        for (row, idx) in filtered[scroll_offset..end].iter().enumerate() {
            let event = &self.events[*idx];
            let y = inner.y + row as u16;
            if y >= inner.bottom() {
                break;
            }

            let is_selected = (scroll_offset + row) == selected;
            let mut style = Style::new().fg(event.severity.color());
            if is_selected {
                style = style
                    .fg(theme::fg::PRIMARY)
                    .bg(theme::alpha::HIGHLIGHT)
                    .bold();
            }

            let line = format!(
                "{:>4} {:<5} {:<7} {:<7} {}",
                event.tick,
                event.severity.label(),
                event.component.label(),
                event.kind.label(),
                event.summary
            );
            Paragraph::new(line).style(style).render(Rect::new(inner.x, y, inner.width, 1), frame);
        }
    }

    fn render_details(&self, frame: &mut Frame, area: Rect, filtered: &[usize]) {
        let border_style = Style::new().fg(theme::screen_accent::ADVANCED);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Event Detail")
            .title_alignment(Alignment::Center)
            .style(border_style);
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        if filtered.is_empty() {
            Paragraph::new("Select an event to inspect.")
                .style(theme::muted())
                .render(inner, frame);
            return;
        }

        let idx = filtered[self.selected.min(filtered.len().saturating_sub(1))];
        let event = &self.events[idx];

        let mut lines = Vec::new();
        lines.push(format!("ID: {}", event.id));
        lines.push(format!("Tick: {}", event.tick));
        lines.push(format!("Severity: {}", event.severity.label()));
        lines.push(format!("Component: {}", event.component.label()));
        lines.push(format!("Type: {}", event.kind.label()));

        lines.push(String::new());
        lines.push("Summary:".to_string());
        lines.push(format!("  {}", event.summary));

        if self.show_details {
            if !event.fields.is_empty() {
                lines.push(String::new());
                lines.push("Fields:".to_string());
                for (k, v) in &event.fields {
                    lines.push(format!("  {k}: {v}"));
                }
            }

            if let Some(evidence) = &event.evidence {
                lines.push(String::new());
                lines.push("Evidence:".to_string());
                lines.push(format!("  {evidence}"));
            }
        } else {
            lines.push(String::new());
            lines.push("Press Enter to expand details".to_string());
        }

        Paragraph::new(lines.join("\n"))
            .style(theme::body())
            .render(inner, frame);
    }
}

impl Screen for ActionTimeline {
    type Message = Event;

    fn update(&mut self, event: &Event) -> Cmd<Self::Message> {
        if let Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match (*code, *modifiers) {
                (KeyCode::Char('f'), Modifiers::NONE) | (KeyCode::Char('F'), Modifiers::NONE) => {
                    self.follow = !self.follow;
                }
                (KeyCode::Char('c'), Modifiers::NONE) | (KeyCode::Char('C'), Modifiers::NONE) => {
                    self.cycle_component();
                    self.sync_selection();
                }
                (KeyCode::Char('s'), Modifiers::NONE) | (KeyCode::Char('S'), Modifiers::NONE) => {
                    self.cycle_severity();
                    self.sync_selection();
                }
                (KeyCode::Char('t'), Modifiers::NONE) | (KeyCode::Char('T'), Modifiers::NONE) => {
                    self.cycle_kind();
                    self.sync_selection();
                }
                (KeyCode::Char('x'), Modifiers::NONE) | (KeyCode::Char('X'), Modifiers::NONE) => {
                    self.clear_filters();
                    self.sync_selection();
                }
                (KeyCode::Enter, _) => {
                    self.show_details = !self.show_details;
                }
                (KeyCode::Up, _) | (KeyCode::Char('k'), Modifiers::NONE) => {
                    self.follow = false;
                    self.selected = self.selected.saturating_sub(1);
                    self.sync_selection();
                }
                (KeyCode::Down, _) | (KeyCode::Char('j'), Modifiers::NONE) => {
                    self.follow = false;
                    self.selected = self.selected.saturating_add(1);
                    self.sync_selection();
                }
                (KeyCode::PageUp, _) => {
                    self.follow = false;
                    self.selected = self.selected.saturating_sub(self.viewport_height);
                    self.sync_selection();
                }
                (KeyCode::PageDown, _) => {
                    self.follow = false;
                    self.selected = self.selected.saturating_add(self.viewport_height);
                    self.sync_selection();
                }
                (KeyCode::Home, _) => {
                    self.follow = false;
                    self.selected = 0;
                    self.sync_selection();
                }
                (KeyCode::End, _) => {
                    self.follow = false;
                    self.selected = usize::MAX / 2;
                    self.sync_selection();
                }
                _ => {}
            }
        }
        Cmd::None
    }

    fn tick(&mut self, tick_count: u64) {
        self.tick_count = tick_count;
        if tick_count % EVENT_BURST_EVERY == 0 {
            self.push_event(self.synthetic_event());
            self.sync_selection();
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let rows = Flex::vertical()
            .constraints([Constraint::Fixed(3), Constraint::Min(1)])
            .split(area);

        let cols = Flex::horizontal()
            .constraints([Constraint::Min(45), Constraint::Min(30)])
            .split(rows[1]);

        let filtered = self.filtered_indices();
        self.render_filters(frame, rows[0]);
        self.render_timeline(frame, cols[0], &filtered);
        self.render_details(frame, cols[1], &filtered);
    }

    fn keybindings(&self) -> Vec<HelpEntry> {
        vec![
            HelpEntry {
                key: "F",
                action: "Toggle follow mode",
            },
            HelpEntry {
                key: "C",
                action: "Cycle component filter",
            },
            HelpEntry {
                key: "S",
                action: "Cycle severity filter",
            },
            HelpEntry {
                key: "T",
                action: "Cycle type filter",
            },
            HelpEntry {
                key: "X",
                action: "Clear filters",
            },
            HelpEntry {
                key: "Enter",
                action: "Toggle detail expansion",
            },
            HelpEntry {
                key: "↑/↓ or j/k",
                action: "Navigate events",
            },
            HelpEntry {
                key: "PgUp/PgDn",
                action: "Page navigation",
            },
        ]
    }

    fn title(&self) -> &'static str {
        "Action Timeline"
    }

    fn tab_label(&self) -> &'static str {
        "Timeline"
    }
}

impl ActionTimeline {
    fn sync_selection(&mut self) {
        let filtered_len = self.filtered_indices().len();
        self.ensure_selection(filtered_len);
    }
}

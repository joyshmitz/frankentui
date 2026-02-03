#![forbid(unsafe_code)]

//! Widget Gallery screen — showcases every available widget type.

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ftui_core::geometry::Rect;
use ftui_layout::{Constraint, Flex};
use ftui_render::frame::Frame;
use ftui_runtime::Cmd;
use ftui_style::{Style, StyleFlags};
use ftui_widgets::Badge;
use ftui_widgets::StatefulWidget;
use ftui_widgets::Widget;
use ftui_widgets::block::{Alignment, Block};
use ftui_widgets::borders::{BorderType, Borders};
use ftui_widgets::columns::{Column, Columns};
use ftui_widgets::json_view::JsonView;
use ftui_widgets::list::{List, ListItem, ListState};
use ftui_widgets::paginator::{Paginator, PaginatorMode};
use ftui_widgets::paragraph::Paragraph;
use ftui_widgets::progress::ProgressBar;
use ftui_widgets::rule::Rule;
use ftui_widgets::scrollbar::{Scrollbar, ScrollbarOrientation, ScrollbarState};
use ftui_widgets::spinner::SpinnerState;
use ftui_widgets::table::{Row, Table};
use ftui_widgets::tree::{Tree, TreeGuides, TreeNode};

use super::{HelpEntry, Screen};
use crate::theme;
use crate::theme::{BadgeSpec, PriorityBadge, StatusBadge};

/// Number of gallery sections.
const SECTION_COUNT: usize = 7;

/// Section names.
const SECTION_NAMES: [&str; SECTION_COUNT] = [
    "A: Borders",
    "B: Text Styles",
    "C: Colors",
    "D: Interactive",
    "E: Data",
    "F: Layout",
    "G: Utility",
];

/// Widget Gallery screen state.
pub struct WidgetGallery {
    current_section: usize,
    tick_count: u64,
    spinner_state: SpinnerState,
    list_state: ListState,
}

impl Default for WidgetGallery {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetGallery {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.selected = Some(0);
        list_state.offset = 0;
        Self {
            current_section: 0,
            tick_count: 0,
            spinner_state: SpinnerState::default(),
            list_state,
        }
    }
}

impl Screen for WidgetGallery {
    type Message = Event;

    fn update(&mut self, event: &Event) -> Cmd<Self::Message> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match code {
                KeyCode::Char('j') | KeyCode::Right => {
                    self.current_section = (self.current_section + 1) % SECTION_COUNT;
                }
                KeyCode::Char('k') | KeyCode::Left => {
                    self.current_section = if self.current_section == 0 {
                        SECTION_COUNT - 1
                    } else {
                        self.current_section - 1
                    };
                }
                _ => {}
            }
        }
        Cmd::None
    }

    fn tick(&mut self, tick_count: u64) {
        self.tick_count = tick_count;
        self.spinner_state.tick();
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.height < 5 || area.width < 20 {
            let msg = Paragraph::new("Terminal too small").style(theme::muted());
            msg.render(area, frame);
            return;
        }

        // Vertical: section tabs (1) + content + paginator (1)
        let v_chunks = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Min(4),
                Constraint::Fixed(1),
            ])
            .split(area);

        self.render_section_tabs(frame, v_chunks[0]);
        self.render_section_content(frame, v_chunks[1]);
        self.render_paginator(frame, v_chunks[2]);
    }

    fn keybindings(&self) -> Vec<HelpEntry> {
        vec![
            HelpEntry {
                key: "j/→",
                action: "Next section",
            },
            HelpEntry {
                key: "k/←",
                action: "Previous section",
            },
        ]
    }

    fn title(&self) -> &'static str {
        "Widget Gallery"
    }

    fn tab_label(&self) -> &'static str {
        "Widgets"
    }
}

impl WidgetGallery {
    fn render_section_tabs(&self, frame: &mut Frame, area: Rect) {
        let mut tab_text = String::new();
        for (i, name) in SECTION_NAMES.iter().enumerate() {
            if i > 0 {
                tab_text.push_str(" │ ");
            }
            if i == self.current_section {
                tab_text.push_str(&format!("▸ {name}"));
            } else {
                tab_text.push_str(&format!("  {name}"));
            }
        }
        let style = if self.current_section < SECTION_COUNT {
            Style::new()
                .fg(theme::screen_accent::WIDGET_GALLERY)
                .attrs(StyleFlags::BOLD)
        } else {
            theme::muted()
        };
        Paragraph::new(tab_text).style(style).render(area, frame);
    }

    fn render_section_content(&self, frame: &mut Frame, area: Rect) {
        match self.current_section {
            0 => self.render_borders(frame, area),
            1 => self.render_text_styles(frame, area),
            2 => self.render_colors(frame, area),
            3 => self.render_interactive(frame, area),
            4 => self.render_data_widgets(frame, area),
            5 => self.render_layout_widgets(frame, area),
            6 => self.render_utility_widgets(frame, area),
            _ => {}
        }
    }

    fn render_paginator(&self, frame: &mut Frame, area: Rect) {
        let pag = Paginator::with_pages((self.current_section as u64) + 1, SECTION_COUNT as u64)
            .mode(PaginatorMode::Dots)
            .style(Style::new().fg(theme::fg::MUTED));
        pag.render(area, frame);
    }

    // -----------------------------------------------------------------------
    // Section A: Borders
    // -----------------------------------------------------------------------
    fn render_borders(&self, frame: &mut Frame, area: Rect) {
        let border_types = [
            ("ASCII", BorderType::Ascii),
            ("Square", BorderType::Square),
            ("Rounded", BorderType::Rounded),
            ("Double", BorderType::Double),
            ("Heavy", BorderType::Heavy),
            ("Custom", BorderType::Rounded),
        ];

        let alignments = [
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
            Alignment::Left,
            Alignment::Center,
            Alignment::Right,
        ];

        let colors = [
            theme::accent::PRIMARY,
            theme::accent::SECONDARY,
            theme::accent::SUCCESS,
            theme::accent::WARNING,
            theme::accent::ERROR,
            theme::accent::INFO,
        ];

        // 2 rows of 3
        let rows = Flex::vertical()
            .constraints([Constraint::Percentage(50.0), Constraint::Percentage(50.0)])
            .split(area);

        for (row_idx, row_area) in rows.iter().enumerate().take(2) {
            let cols = Flex::horizontal()
                .constraints([
                    Constraint::Percentage(33.3),
                    Constraint::Percentage(33.3),
                    Constraint::Percentage(33.4),
                ])
                .split(*row_area);

            for (col_idx, col_area) in cols.iter().enumerate().take(3) {
                let i = row_idx * 3 + col_idx;
                let (name, bt) = border_types[i];
                let block = Block::new()
                    .borders(Borders::ALL)
                    .border_type(bt)
                    .title(name)
                    .title_alignment(alignments[i])
                    .style(Style::new().fg(colors[i]));
                let inner = block.inner(*col_area);
                block.render(*col_area, frame);

                let desc = format!("Border: {name}\nAlign: {:?}", alignments[i]);
                Paragraph::new(desc)
                    .style(theme::body())
                    .render(inner, frame);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Section B: Text Styles
    // -----------------------------------------------------------------------
    fn render_text_styles(&self, frame: &mut Frame, area: Rect) {
        let styles: Vec<(&str, Style)> = vec![
            ("Bold", theme::bold()),
            ("Dim", theme::dim()),
            ("Italic", theme::italic()),
            ("Underline", theme::underline()),
            ("DblUnder", theme::double_underline()),
            ("CurlyUL", theme::curly_underline()),
            ("Blink", theme::blink_style()),
            ("Reverse", theme::reverse()),
            ("Hidden", theme::hidden()),
            ("Strike", theme::strikethrough()),
            (
                "Bold+Italic",
                Style::new()
                    .fg(theme::accent::PRIMARY)
                    .attrs(StyleFlags::BOLD | StyleFlags::ITALIC),
            ),
            (
                "Bold+Under",
                Style::new()
                    .fg(theme::accent::SECONDARY)
                    .attrs(StyleFlags::BOLD | StyleFlags::UNDERLINE),
            ),
            (
                "Dim+Italic",
                Style::new()
                    .fg(theme::accent::SUCCESS)
                    .attrs(StyleFlags::DIM | StyleFlags::ITALIC),
            ),
            (
                "All Flags",
                Style::new().fg(theme::accent::WARNING).attrs(
                    StyleFlags::BOLD
                        | StyleFlags::ITALIC
                        | StyleFlags::UNDERLINE
                        | StyleFlags::STRIKETHROUGH,
                ),
            ),
        ];

        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Text Style Flags")
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        // Grid: fill rows, ~3 columns
        let col_width = 16u16;
        let cols_per_row = (inner.width / col_width).max(1) as usize;

        for (i, (label, style)) in styles.iter().enumerate() {
            let row = (i / cols_per_row) as u16;
            let col = (i % cols_per_row) as u16;
            let x = inner.x + col * col_width;
            let y = inner.y + row;
            if y >= inner.y + inner.height {
                break;
            }
            let cell_area = Rect {
                x,
                y,
                width: col_width.min(inner.x + inner.width - x),
                height: 1,
            };
            Paragraph::new(*label)
                .style(*style)
                .render(cell_area, frame);
        }
    }

    // -----------------------------------------------------------------------
    // Section C: Colors
    // -----------------------------------------------------------------------
    fn render_colors(&self, frame: &mut Frame, area: Rect) {
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(3),
                Constraint::Fixed(1),
                Constraint::Min(2),
                Constraint::Fixed(1),
                Constraint::Fixed(3),
            ])
            .split(area);

        // TrueColor gradient strip
        self.render_color_gradient(frame, rows[0]);

        // Separator: named colors
        Rule::new()
            .title("Named Colors")
            .title_alignment(Alignment::Center)
            .style(theme::muted())
            .render(rows[1], frame);

        // Named accent colors
        self.render_named_colors(frame, rows[2]);

        // Separator: semantic badges
        Rule::new()
            .title("Semantic Badges")
            .title_alignment(Alignment::Center)
            .style(theme::muted())
            .render(rows[3], frame);

        self.render_badges(frame, rows[4]);
    }

    fn render_color_gradient(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Theme Accent Gradient")
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.height == 0 || inner.width == 0 {
            return;
        }

        let w = inner.width as usize;
        for i in 0..w {
            let t = i as f64 / w.max(1) as f64;
            let color = theme::accent_gradient(t);
            // Write each cell with its color using the frame buffer directly
            let x = inner.x + i as u16;
            if x < inner.x + inner.width {
                let cell_area = Rect {
                    x,
                    y: inner.y,
                    width: 1,
                    height: 1,
                };
                Paragraph::new("█")
                    .style(Style::new().fg(color))
                    .render(cell_area, frame);
            }
        }
    }

    fn render_named_colors(&self, frame: &mut Frame, area: Rect) {
        let named = [
            ("Primary", theme::accent::PRIMARY),
            ("Secondary", theme::accent::SECONDARY),
            ("Success", theme::accent::SUCCESS),
            ("Warning", theme::accent::WARNING),
            ("Error", theme::accent::ERROR),
            ("Info", theme::accent::INFO),
            ("Link", theme::accent::LINK),
            ("FG Primary", theme::fg::PRIMARY),
            ("FG Secondary", theme::fg::SECONDARY),
            ("FG Muted", theme::fg::MUTED),
        ];

        let col_width = 16u16;
        let cols_per_row = (area.width / col_width).max(1) as usize;

        for (i, (label, color)) in named.iter().enumerate() {
            let row = (i / cols_per_row) as u16;
            let col = (i % cols_per_row) as u16;
            let x = area.x + col * col_width;
            let y = area.y + row;
            if y >= area.y + area.height {
                break;
            }
            let cell_area = Rect {
                x,
                y,
                width: col_width.min(area.x + area.width - x),
                height: 1,
            };
            let text = format!("██ {label}");
            Paragraph::new(text)
                .style(Style::new().fg(*color))
                .render(cell_area, frame);
        }
    }

    fn render_badges(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        let status_row = Rect::new(area.x, area.y, area.width, 1);
        let priority_row = Rect::new(area.x, area.y.saturating_add(1), area.width, 1);

        let status_badges = [
            theme::status_badge(StatusBadge::Open),
            theme::status_badge(StatusBadge::InProgress),
            theme::status_badge(StatusBadge::Blocked),
            theme::status_badge(StatusBadge::Closed),
        ];

        let priority_badges = [
            theme::priority_badge(PriorityBadge::P0),
            theme::priority_badge(PriorityBadge::P1),
            theme::priority_badge(PriorityBadge::P2),
            theme::priority_badge(PriorityBadge::P3),
            theme::priority_badge(PriorityBadge::P4),
        ];

        self.render_badge_row(frame, status_row, &status_badges);
        self.render_badge_row(frame, priority_row, &priority_badges);
    }

    fn render_badge_row(&self, frame: &mut Frame, area: Rect, badges: &[BadgeSpec]) {
        if area.is_empty() {
            return;
        }

        let mut x = area.x;
        let max_x = area.right();
        let y = area.y;

        for badge_spec in badges {
            let badge = Badge::new(badge_spec.label).with_style(badge_spec.style);
            let w = badge.width().min(area.width);
            if x >= max_x || x.saturating_add(w) > max_x {
                break;
            }
            badge.render(Rect::new(x, y, w, 1), frame);
            x = x.saturating_add(w).saturating_add(1);
        }
    }

    // -----------------------------------------------------------------------
    // Section D: Interactive
    // -----------------------------------------------------------------------
    fn render_interactive(&self, frame: &mut Frame, area: Rect) {
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(3),
                Constraint::Percentage(40.0),
                Constraint::Percentage(40.0),
            ])
            .split(area);

        // TextInput demos
        self.render_text_inputs(frame, rows[0]);
        // List and Table side by side
        self.render_list_and_table(frame, rows[1]);
        // Tree widget
        self.render_tree(frame, rows[2]);
    }

    fn render_text_inputs(&self, frame: &mut Frame, area: Rect) {
        let cols = Flex::horizontal()
            .constraints([Constraint::Percentage(50.0), Constraint::Percentage(50.0)])
            .split(area);

        // Plain text input
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("TextInput")
            .style(theme::content_border());
        let inner = block.inner(cols[0]);
        block.render(cols[0], frame);
        Paragraph::new("demo@example.com")
            .style(Style::new().fg(theme::fg::PRIMARY))
            .render(inner, frame);

        // Masked input
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Masked Input")
            .style(theme::content_border());
        let inner = block.inner(cols[1]);
        block.render(cols[1], frame);
        Paragraph::new("••••••••")
            .style(Style::new().fg(theme::fg::MUTED))
            .render(inner, frame);
    }

    fn render_list_and_table(&self, frame: &mut Frame, area: Rect) {
        let cols = Flex::horizontal()
            .constraints([Constraint::Percentage(40.0), Constraint::Percentage(60.0)])
            .split(area);

        // List
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("List")
            .style(theme::content_border());
        let inner = block.inner(cols[0]);
        block.render(cols[0], frame);

        let items: Vec<ListItem> = (0..10)
            .map(|i| {
                let style = if Some(i) == self.list_state.selected {
                    Style::new()
                        .fg(theme::accent::PRIMARY)
                        .attrs(StyleFlags::BOLD)
                } else {
                    Style::new().fg(theme::fg::SECONDARY)
                };
                ListItem::new(format!("Item {}", i + 1)).style(style)
            })
            .collect();
        Widget::render(
            &List::new(items).style(Style::new().fg(theme::fg::SECONDARY)),
            inner,
            frame,
        );

        // Table
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Table")
            .style(theme::content_border());
        let inner = block.inner(cols[1]);
        block.render(cols[1], frame);

        let header = Row::new(["Name", "Type", "Size", "Modified"]).style(theme::title());
        let table_rows: Vec<Row> = vec![
            Row::new(["main.rs", "Rust", "4.2 KB", "2m ago"]),
            Row::new(["lib.rs", "Rust", "12.8 KB", "5m ago"]),
            Row::new(["config.toml", "TOML", "1.1 KB", "1h ago"]),
            Row::new(["README.md", "Markdown", "3.4 KB", "2d ago"]),
            Row::new(["Cargo.lock", "Lock", "89 KB", "2m ago"]),
        ];
        let widths = [
            Constraint::Min(10),
            Constraint::Fixed(8),
            Constraint::Fixed(8),
            Constraint::Fixed(10),
        ];
        Widget::render(
            &Table::new(table_rows, widths)
                .header(header)
                .style(Style::new().fg(theme::fg::SECONDARY)),
            inner,
            frame,
        );
    }

    fn render_tree(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Tree (Unicode Guides)")
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        let root = TreeNode::new("project/")
            .child(
                TreeNode::new("src/")
                    .child(TreeNode::new("main.rs"))
                    .child(TreeNode::new("lib.rs"))
                    .child(
                        TreeNode::new("screens/")
                            .child(TreeNode::new("dashboard.rs"))
                            .child(TreeNode::new("gallery.rs")),
                    ),
            )
            .child(TreeNode::new("tests/").child(TreeNode::new("integration.rs")))
            .child(TreeNode::new("Cargo.toml"));
        let tree = Tree::new(root)
            .with_guides(TreeGuides::Unicode)
            .with_label_style(Style::new().fg(theme::fg::PRIMARY));
        tree.render(inner, frame);
    }

    // -----------------------------------------------------------------------
    // Section E: Data Widgets
    // -----------------------------------------------------------------------
    fn render_data_widgets(&self, frame: &mut Frame, area: Rect) {
        let rows = Flex::vertical()
            .constraints([Constraint::Fixed(7), Constraint::Min(4)])
            .split(area);

        // Progress bars
        self.render_progress_bars(frame, rows[0]);
        // JSON view and spinner
        self.render_json_and_spinner(frame, rows[1]);
    }

    fn render_progress_bars(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("ProgressBar")
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        if inner.height == 0 {
            return;
        }

        let ratios = [0.0, 0.25, 0.50, 0.75, 1.0];
        let colors = [
            theme::accent::ERROR,
            theme::accent::WARNING,
            theme::accent::INFO,
            theme::accent::PRIMARY,
            theme::accent::SUCCESS,
        ];

        let bar_rows = Flex::vertical()
            .constraints(
                ratios
                    .iter()
                    .map(|_| Constraint::Fixed(1))
                    .collect::<Vec<_>>(),
            )
            .split(inner);

        for (i, (&ratio, &color)) in ratios.iter().zip(colors.iter()).enumerate() {
            if i >= bar_rows.len() {
                break;
            }
            let pct = (ratio * 100.0) as u32;
            let label = format!("{pct}%");
            ProgressBar::new()
                .ratio(ratio)
                .label(&label)
                .style(Style::new().fg(theme::fg::MUTED))
                .gauge_style(Style::new().fg(color).bg(theme::alpha::SURFACE))
                .render(bar_rows[i], frame);
        }
    }

    fn render_json_and_spinner(&self, frame: &mut Frame, area: Rect) {
        let cols = Flex::horizontal()
            .constraints([Constraint::Percentage(65.0), Constraint::Percentage(35.0)])
            .split(area);

        // JsonView
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("JsonView")
            .style(theme::content_border());
        let inner = block.inner(cols[0]);
        block.render(cols[0], frame);

        let sample_json = r#"{"name": "FrankenTUI", "version": "0.1.0", "widgets": 28, "features": ["charts", "forms", "canvas"], "nested": {"key": "value"}}"#;
        JsonView::new(sample_json)
            .with_indent(2)
            .with_key_style(
                Style::new()
                    .fg(theme::accent::PRIMARY)
                    .attrs(StyleFlags::BOLD),
            )
            .with_string_style(Style::new().fg(theme::accent::SUCCESS))
            .with_number_style(Style::new().fg(theme::accent::WARNING))
            .with_literal_style(Style::new().fg(theme::accent::ERROR))
            .with_punct_style(Style::new().fg(theme::fg::MUTED))
            .render(inner, frame);

        // Spinner
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Spinners")
            .style(theme::content_border());
        let inner = block.inner(cols[1]);
        block.render(cols[1], frame);

        if inner.height >= 2 {
            // Show dots spinner on first row
            let top_row = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: 1,
            };
            let dots_idx = self.spinner_state.current_frame % ftui_widgets::spinner::DOTS.len();
            let dots_frame = ftui_widgets::spinner::DOTS[dots_idx];
            Paragraph::new(format!("{dots_frame} Loading (DOTS)"))
                .style(Style::new().fg(theme::accent::PRIMARY))
                .render(top_row, frame);

            // Show line spinner on second row
            if inner.height >= 2 {
                let bot_row = Rect {
                    x: inner.x,
                    y: inner.y + 1,
                    width: inner.width,
                    height: 1,
                };
                let line_idx = self.spinner_state.current_frame % ftui_widgets::spinner::LINE.len();
                let line_frame = ftui_widgets::spinner::LINE[line_idx];
                Paragraph::new(format!("{line_frame} Processing (LINE)"))
                    .style(Style::new().fg(theme::accent::SECONDARY))
                    .render(bot_row, frame);
            }

            // Show tick count
            if inner.height >= 4 {
                let info_row = Rect {
                    x: inner.x,
                    y: inner.y + 3,
                    width: inner.width,
                    height: 1,
                };
                Paragraph::new(format!("Frame: {}", self.spinner_state.current_frame))
                    .style(theme::muted())
                    .render(info_row, frame);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Section F: Layout Widgets
    // -----------------------------------------------------------------------
    fn render_layout_widgets(&self, frame: &mut Frame, area: Rect) {
        let rows = Flex::vertical()
            .constraints([
                Constraint::Percentage(40.0),
                Constraint::Percentage(30.0),
                Constraint::Percentage(30.0),
            ])
            .split(area);

        // Columns demo
        self.render_columns_demo(frame, rows[0]);
        // Flex demo
        self.render_flex_demo(frame, rows[1]);
        // Padding demo
        self.render_padding_demo(frame, rows[2]);
    }

    fn render_columns_demo(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Columns Widget")
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        let col1 =
            Paragraph::new("Column 1\nFixed(15)").style(Style::new().fg(theme::accent::PRIMARY));
        let col2 =
            Paragraph::new("Column 2\nMin(10)").style(Style::new().fg(theme::accent::SECONDARY));
        let col3 = Paragraph::new("Column 3\nPercentage(40%)")
            .style(Style::new().fg(theme::accent::SUCCESS));

        let columns = Columns::new()
            .push(Column::new(col1, Constraint::Fixed(15)))
            .push(Column::new(col2, Constraint::Min(10)))
            .push(Column::new(col3, Constraint::Percentage(40.0)))
            .gap(theme::spacing::XS);
        columns.render(inner, frame);
    }

    fn render_flex_demo(&self, frame: &mut Frame, area: Rect) {
        let cols = Flex::horizontal()
            .constraints([Constraint::Percentage(50.0), Constraint::Percentage(50.0)])
            .split(area);

        // Horizontal flex
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Flex Horizontal")
            .style(theme::content_border());
        let inner = block.inner(cols[0]);
        block.render(cols[0], frame);

        let h_chunks = Flex::horizontal()
            .constraints([
                Constraint::Fixed(8),
                Constraint::Min(4),
                Constraint::Fixed(8),
            ])
            .split(inner);
        for (i, &color) in [
            theme::accent::PRIMARY,
            theme::accent::SECONDARY,
            theme::accent::SUCCESS,
        ]
        .iter()
        .enumerate()
        {
            if i < h_chunks.len() {
                Paragraph::new(format!("H{}", i + 1))
                    .style(Style::new().fg(color).attrs(StyleFlags::BOLD))
                    .render(h_chunks[i], frame);
            }
        }

        // Vertical flex
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Flex Vertical")
            .style(theme::content_border());
        let inner = block.inner(cols[1]);
        block.render(cols[1], frame);

        let v_chunks = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Min(1),
                Constraint::Fixed(1),
            ])
            .split(inner);
        for (i, &color) in [
            theme::accent::WARNING,
            theme::accent::ERROR,
            theme::accent::INFO,
        ]
        .iter()
        .enumerate()
        {
            if i < v_chunks.len() {
                Paragraph::new(format!("V{}", i + 1))
                    .style(Style::new().fg(color).attrs(StyleFlags::BOLD))
                    .render(v_chunks[i], frame);
            }
        }
    }

    fn render_padding_demo(&self, frame: &mut Frame, area: Rect) {
        let pad_x = theme::spacing::SM;
        let pad_y = theme::spacing::XS;

        let title = format!("Padding ({pad_x},{pad_x},{pad_y},{pad_y})");
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title.as_str())
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        // Apply padding manually to show the concept
        let padded = Rect {
            x: inner.x + pad_x,
            y: inner.y + pad_y,
            width: inner.width.saturating_sub(pad_x * 2),
            height: inner.height.saturating_sub(pad_y * 2),
        };
        Paragraph::new(format!(
            "Content inside padding.\nPadding: left={pad_x}, right={pad_x}, top={pad_y}, bottom={pad_y}"
        ))
            .style(Style::new().fg(theme::accent::INFO))
            .render(padded, frame);
    }

    // -----------------------------------------------------------------------
    // Section G: Utility Widgets
    // -----------------------------------------------------------------------
    fn render_utility_widgets(&self, frame: &mut Frame, area: Rect) {
        let rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(1),
                Constraint::Fixed(3),
                Constraint::Min(3),
            ])
            .split(area);

        // Rule
        Rule::new()
            .title("Horizontal Rule")
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(theme::accent::SECONDARY))
            .render(rows[0], frame);

        // Scrollbar demo
        self.render_scrollbar_demo(frame, rows[1]);

        // Paginator modes + summary
        self.render_paginator_modes(frame, rows[2]);
    }

    fn render_scrollbar_demo(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Scrollbar (V+H)")
            .style(theme::content_border());
        let inner = block.inner(area);
        block.render(area, frame);

        // Vertical scrollbar on right edge
        if inner.width > 2 && inner.height > 0 {
            let v_area = Rect {
                x: inner.x + inner.width - 1,
                y: inner.y,
                width: 1,
                height: inner.height,
            };
            let mut v_state = ScrollbarState::new(100, 33, inner.height as usize);
            let v_sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::new().fg(theme::accent::PRIMARY));
            StatefulWidget::render(&v_sb, v_area, frame, &mut v_state);

            // Horizontal scrollbar on bottom
            if inner.height >= 2 {
                let h_area = Rect {
                    x: inner.x,
                    y: inner.y + inner.height - 1,
                    width: inner.width.saturating_sub(1),
                    height: 1,
                };
                let mut h_state =
                    ScrollbarState::new(200, 75, inner.width.saturating_sub(1) as usize);
                let h_sb = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
                    .thumb_style(Style::new().fg(theme::accent::SECONDARY));
                StatefulWidget::render(&h_sb, h_area, frame, &mut h_state);
            }
        }
    }

    fn render_paginator_modes(&self, frame: &mut Frame, area: Rect) {
        let cols = Flex::horizontal()
            .constraints([
                Constraint::Percentage(33.3),
                Constraint::Percentage(33.3),
                Constraint::Percentage(33.4),
            ])
            .split(area);

        // Page mode
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Paginator: Page")
            .style(theme::content_border());
        let inner = block.inner(cols[0]);
        block.render(cols[0], frame);
        Paginator::with_pages(2, 5)
            .mode(PaginatorMode::Page)
            .style(Style::new().fg(theme::accent::PRIMARY))
            .render(inner, frame);

        // Compact mode
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Paginator: Compact")
            .style(theme::content_border());
        let inner = block.inner(cols[1]);
        block.render(cols[1], frame);
        Paginator::with_pages(3, 5)
            .mode(PaginatorMode::Compact)
            .style(Style::new().fg(theme::accent::SECONDARY))
            .render(inner, frame);

        // Dots mode
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Paginator: Dots")
            .style(theme::content_border());
        let inner = block.inner(cols[2]);
        block.render(cols[2], frame);
        Paginator::with_pages(4, 5)
            .mode(PaginatorMode::Dots)
            .style(Style::new().fg(theme::accent::SUCCESS))
            .render(inner, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gallery_initial_state() {
        let gallery = WidgetGallery::new();
        assert_eq!(gallery.current_section, 0);
        assert_eq!(gallery.tick_count, 0);
    }

    #[test]
    fn gallery_section_navigation() {
        let mut gallery = WidgetGallery::new();
        assert_eq!(gallery.current_section, 0);

        // Navigate forward with j
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: Default::default(),
            kind: KeyEventKind::Press,
        });
        gallery.update(&ev);
        assert_eq!(gallery.current_section, 1);

        // Navigate forward again
        gallery.update(&ev);
        assert_eq!(gallery.current_section, 2);

        // Navigate backward with k
        let ev_back = Event::Key(KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: Default::default(),
            kind: KeyEventKind::Press,
        });
        gallery.update(&ev_back);
        assert_eq!(gallery.current_section, 1);
    }

    #[test]
    fn gallery_section_wrap_around() {
        let mut gallery = WidgetGallery::new();

        // Navigate backward from 0 wraps to last section
        let ev_back = Event::Key(KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: Default::default(),
            kind: KeyEventKind::Press,
        });
        gallery.update(&ev_back);
        assert_eq!(gallery.current_section, SECTION_COUNT - 1);

        // Navigate forward from last wraps to 0
        let ev_fwd = Event::Key(KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: Default::default(),
            kind: KeyEventKind::Press,
        });
        gallery.update(&ev_fwd);
        assert_eq!(gallery.current_section, 0);
    }

    #[test]
    fn gallery_all_borders() {
        // Verify all 6 border types are distinct
        let types = [
            BorderType::Ascii,
            BorderType::Square,
            BorderType::Rounded,
            BorderType::Double,
            BorderType::Heavy,
        ];
        // All should be distinct enum variants
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j]);
            }
        }
    }

    #[test]
    fn gallery_all_styles() {
        // Verify style flag combos produce distinct styles
        let flags = [
            StyleFlags::BOLD,
            StyleFlags::DIM,
            StyleFlags::ITALIC,
            StyleFlags::UNDERLINE,
            StyleFlags::DOUBLE_UNDERLINE,
            StyleFlags::CURLY_UNDERLINE,
            StyleFlags::BLINK,
            StyleFlags::REVERSE,
            StyleFlags::HIDDEN,
            StyleFlags::STRIKETHROUGH,
        ];
        // All single flags should be distinct
        for i in 0..flags.len() {
            for j in (i + 1)..flags.len() {
                assert_ne!(flags[i], flags[j]);
            }
        }
    }

    #[test]
    fn gallery_tick_updates_spinner() {
        let mut gallery = WidgetGallery::new();
        assert_eq!(gallery.spinner_state.current_frame, 0);
        gallery.tick(1);
        assert_eq!(gallery.spinner_state.current_frame, 1);
        gallery.tick(2);
        assert_eq!(gallery.spinner_state.current_frame, 2);
    }
}

use crate::block::Block;
use crate::mouse::MouseResult;
use crate::undo_support::{TableUndoExt, UndoSupport, UndoWidgetId};
use crate::{
    MeasurableWidget, SizeConstraints, StatefulWidget, Widget, apply_style, set_style_area,
};
use ftui_core::event::{MouseButton, MouseEvent, MouseEventKind};
use ftui_core::geometry::{Rect, Size};
use ftui_layout::{Constraint, Flex};
use ftui_render::buffer::Buffer;
use ftui_render::cell::Cell;
use ftui_render::frame::{Frame, HitId, HitRegion};
use ftui_style::{
    Style, TableEffectResolver, TableEffectScope, TableEffectTarget, TableSection, TableTheme,
};
use ftui_text::{Line, Span, Text};
use std::any::Any;

fn text_into_owned(text: Text<'_>) -> Text<'static> {
    Text::from_lines(
        text.into_iter()
            .map(|line| Line::from_spans(line.into_iter().map(Span::into_owned))),
    )
}

/// A row in a table.
#[derive(Debug, Clone, Default)]
pub struct Row {
    cells: Vec<Text<'static>>,
    height: u16,
    style: Style,
    bottom_margin: u16,
}

impl Row {
    /// Create a new row from an iterator of cell contents.
    #[must_use]
    pub fn new<'a>(cells: impl IntoIterator<Item = impl Into<Text<'a>>>) -> Self {
        Self {
            cells: cells
                .into_iter()
                .map(|c| text_into_owned(c.into()))
                .collect(),
            height: 1,
            style: Style::default(),
            bottom_margin: 0,
        }
    }

    /// Set the row height in lines.
    #[must_use]
    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    /// Set the row style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the bottom margin after this row.
    #[must_use]
    pub fn bottom_margin(mut self, margin: u16) -> Self {
        self.bottom_margin = margin;
        self
    }
}

/// A widget to display data in a table.
#[derive(Debug, Clone, Default)]
pub struct Table<'a> {
    rows: Vec<Row>,
    widths: Vec<Constraint>,
    header: Option<Row>,
    block: Option<Block<'a>>,
    style: Style,
    highlight_style: Style,
    theme: TableTheme,
    theme_phase: f32,
    column_spacing: u16,
    /// Optional hit ID for mouse interaction.
    /// When set, each table row registers a hit region with the hit grid.
    hit_id: Option<HitId>,
    /// Optional data hash to enable caching of filtered and sorted indices.
    data_hash: Option<u64>,
}

impl<'a> Table<'a> {
    /// Create a new table with the given rows and column width constraints.
    #[must_use]
    pub fn new(
        rows: impl IntoIterator<Item = Row>,
        widths: impl IntoIterator<Item = Constraint>,
    ) -> Self {
        let rows: Vec<Row> = rows.into_iter().collect();
        let widths: Vec<Constraint> = widths.into_iter().collect();

        Self {
            rows,
            widths,
            header: None,
            block: None,
            style: Style::default(),
            highlight_style: Style::default(),
            theme: TableTheme::default(),
            theme_phase: 0.0,
            column_spacing: 1,
            hit_id: None,
            data_hash: None,
        }
    }

    /// Set an explicit data hash to enable caching of filtered and sorted indices.
    ///
    /// This is highly recommended for large tables. When provided, the table widget
    /// will cache the result of filtering and sorting in the `TableState`, skipping
    /// expensive O(N) re-evaluation on frames where the hash, filter, and sort
    /// parameters have not changed.
    #[must_use]
    pub fn data_hash(mut self, hash: u64) -> Self {
        self.data_hash = Some(hash);
        self
    }

    /// Set the header row.
    #[must_use]
    pub fn header(mut self, header: Row) -> Self {
        self.header = Some(header);
        self
    }

    /// Set the surrounding block.
    #[must_use]
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set the base table style.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the style for the selected row.
    #[must_use]
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Set the table theme (base/states/effects).
    #[must_use]
    pub fn theme(mut self, theme: TableTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the explicit animation phase for theme effects.
    ///
    /// Phase is deterministic and should be supplied by the caller (e.g. from tick count).
    #[must_use]
    pub fn theme_phase(mut self, phase: f32) -> Self {
        self.theme_phase = phase;
        self
    }

    /// Set the spacing between columns.
    #[must_use]
    pub fn column_spacing(mut self, spacing: u16) -> Self {
        self.column_spacing = spacing;
        self
    }

    /// Set a hit ID for mouse interaction.
    ///
    /// When set, each table row will register a hit region with the frame's
    /// hit grid (if enabled). The hit data will be the row's index, allowing
    /// click handlers to determine which row was clicked.
    #[must_use]
    pub fn hit_id(mut self, id: HitId) -> Self {
        self.hit_id = Some(id);
        self
    }

    fn filtered_and_sorted_indices(&self, state: &mut TableState) -> std::sync::Arc<[usize]> {
        if let Some(hash) = self.data_hash
            && let Some((
                cached_hash,
                cached_filter,
                cached_sort_col,
                cached_sort_asc,
                indices,
            )) = &state.cached_display_indices
            && *cached_hash == hash
            && *cached_filter == state.filter
            && *cached_sort_col == state.sort_column
            && *cached_sort_asc == state.sort_ascending
        {
            return std::sync::Arc::clone(indices);
        }

        let mut indices: Vec<usize> = (0..self.rows.len()).collect();

        // 1. Filter
        if !state.filter.trim().is_empty() {
            let query = state.filter.trim().to_lowercase();
            indices.retain(|&i| {
                let row = &self.rows[i];
                row.cells.iter().any(|cell| {
                    // Optimization: check single-span content directly to avoid allocation
                    // from to_plain_text().
                    if let Some(line) = cell.lines().first()
                        && cell.lines().len() == 1
                        && line.spans().len() == 1
                    {
                        return crate::contains_ignore_case(&line.spans()[0].content, &query);
                    }
                    crate::contains_ignore_case(&cell.to_plain_text(), &query)
                })
            });
        }

        // 2. Sort
        if let Some(col_idx) = state.sort_column {
            use std::borrow::Cow;
            let mut sort_keys: Vec<(usize, Cow<str>)> = indices
                .iter()
                .map(|&i| {
                    let cell_text = self.rows[i].cells.get(col_idx);
                    let key = match cell_text {
                        Some(text) => {
                            // Optimization: Borrow content directly if simple (1 line, 1 span)
                            if let Some(line) = text.lines().first() {
                                if text.lines().len() == 1 && line.spans().len() == 1 {
                                    Cow::Borrowed(line.spans()[0].content.as_ref())
                                } else {
                                    Cow::Owned(text.to_plain_text())
                                }
                            } else {
                                Cow::Borrowed("")
                            }
                        }
                        None => Cow::Borrowed(""),
                    };
                    (i, key)
                })
                .collect();

            if state.sort_ascending {
                sort_keys.sort_by(|a, b| a.1.cmp(&b.1));
            } else {
                sort_keys.sort_by(|a, b| b.1.cmp(&a.1));
            }

            indices = sort_keys.into_iter().map(|(i, _)| i).collect();
        }

        let arc_indices: std::sync::Arc<[usize]> = indices.into();

        if let Some(hash) = self.data_hash {
            state.cached_display_indices = Some((
                hash,
                state.filter.clone(),
                state.sort_column,
                state.sort_ascending,
                std::sync::Arc::clone(&arc_indices),
            ));
        }

        arc_indices
    }

    fn requires_measurement(constraints: &[Constraint]) -> bool {
        constraints.iter().any(|c| {
            matches!(
                c,
                Constraint::FitContent | Constraint::FitContentBounded { .. } | Constraint::FitMin
            )
        })
    }

    fn compute_intrinsic_widths(rows: &[Row], header: Option<&Row>, col_count: usize) -> Vec<u16> {
        if col_count == 0 {
            return Vec::new();
        }

        let mut col_widths: Vec<u16> = vec![0; col_count];

        if let Some(header) = header {
            for (i, cell) in header.cells.iter().enumerate().take(col_count) {
                let cell_width = cell
                    .lines()
                    .iter()
                    .take(header.height as usize)
                    .map(|l| l.width())
                    .max()
                    .unwrap_or(0)
                    .min(u16::MAX as usize) as u16;
                col_widths[i] = col_widths[i].max(cell_width);
            }
        }

        for row in rows {
            for (i, cell) in row.cells.iter().enumerate().take(col_count) {
                let cell_width = cell
                    .lines()
                    .iter()
                    .take(row.height as usize)
                    .map(|l| l.width())
                    .max()
                    .unwrap_or(0)
                    .min(u16::MAX as usize) as u16;
                col_widths[i] = col_widths[i].max(cell_width);
            }
        }

        col_widths
    }
}

impl<'a> Widget for Table<'a> {
    fn render(&self, area: Rect, frame: &mut Frame) {
        let mut state = TableState::default();
        StatefulWidget::render(self, area, frame, &mut state);
    }
}

pub type CachedTableDisplayIndices = (u64, String, Option<usize>, bool, std::sync::Arc<[usize]>);

/// Mutable state for a [`Table`] widget.
#[derive(Debug, Clone, Default)]
pub struct TableState {
    /// Unique ID for undo tracking.
    #[allow(dead_code)]
    undo_id: UndoWidgetId,
    /// Index of the currently selected row, if any.
    pub selected: Option<usize>,
    /// Index of the currently hovered row, if any.
    pub hovered: Option<usize>,
    /// Scroll offset (first visible row index).
    pub offset: usize,
    /// Optional persistence ID for state saving/restoration.
    /// When set, this state can be persisted via the [`Stateful`] trait.
    persistence_id: Option<String>,
    /// Current sort column (for undo support).
    pub sort_column: Option<usize>,
    /// Sort ascending (for undo support).
    pub sort_ascending: bool,
    /// Filter text (for undo support).
    pub filter: String,
    /// Cache for stable layout resizing (temporal coherence).
    coherence: ftui_layout::CoherenceCache,
    /// Cached display indices (data_hash, filter, sort_column, sort_ascending, indices)
    #[doc(hidden)]
    pub cached_display_indices: Option<CachedTableDisplayIndices>,
    /// Cached intrinsic column widths (data_hash, widths)
    #[doc(hidden)]
    pub cached_intrinsic_widths: Option<(u64, std::sync::Arc<[u16]>)>,
}

pub type CachedDisplayIndices = (u64, String, Option<usize>, bool, std::sync::Arc<[usize]>);

impl TableState {
    /// Set the selected row index.
    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// Create a new TableState with a persistence ID for state saving.
    #[must_use]
    pub fn with_persistence_id(mut self, id: impl Into<String>) -> Self {
        self.persistence_id = Some(id.into());
        self
    }

    /// Get the persistence ID, if set.
    #[must_use = "use the persistence id (if any)"]
    pub fn persistence_id(&self) -> Option<&str> {
        self.persistence_id.as_deref()
    }
}


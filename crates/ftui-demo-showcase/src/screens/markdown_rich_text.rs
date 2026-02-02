#![forbid(unsafe_code)]

//! Markdown and Rich Text screen — typography and text processing.
//!
//! Demonstrates:
//! - `MarkdownRenderer` with custom `MarkdownTheme`
//! - GFM auto-detection with `is_likely_markdown`
//! - Streaming/fragment rendering with `render_streaming`
//! - Text style attributes (bold, italic, underline, etc.)
//! - Unicode text with CJK and emoji in a `Table`
//! - `WrapMode` and `Alignment` cycling

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ftui_core::geometry::Rect;
use ftui_extras::markdown::{MarkdownRenderer, MarkdownTheme, is_likely_markdown};
use ftui_layout::{Constraint, Flex};
use ftui_render::frame::Frame;
use ftui_runtime::Cmd;
use ftui_style::Style;
use ftui_text::WrapMode;
use ftui_text::text::{Line, Span, Text};
use ftui_widgets::Widget;
use ftui_widgets::block::{Alignment, Block};
use ftui_widgets::borders::{BorderType, Borders};
use ftui_widgets::paragraph::Paragraph;
use ftui_widgets::rule::Rule;
use ftui_widgets::table::{Row, Table};

use super::{HelpEntry, Screen};
use crate::theme;

/// Simulated LLM streaming response with complex GFM content.
/// This demonstrates real-world markdown that an LLM might generate.
const STREAMING_MARKDOWN: &str = "\
# Understanding Quantum Computing

Quantum computers leverage **quantum mechanics** to process information in fundamentally different ways than classical computers.

## Key Concepts

### Qubits vs Classical Bits

While classical bits are either $0$ or $1$, qubits can exist in a **superposition**:

$$|\\psi\\rangle = \\alpha|0\\rangle + \\beta|1\\rangle$$

where $|\\alpha|^2 + |\\beta|^2 = 1$.

### Quantum Gates

Common single-qubit gates include:

| Gate | Matrix | Effect |
|------|--------|--------|
| Hadamard | $H$ | Creates superposition |
| Pauli-X | $X$ | Bit flip (NOT) |
| Pauli-Z | $Z$ | Phase flip |

> [!NOTE]
> The Hadamard gate is fundamental to quantum algorithms like Grover's search.

### Example: Bell State

```python
from qiskit import QuantumCircuit

# Create a Bell state (maximally entangled)
qc = QuantumCircuit(2)
qc.h(0)      # Hadamard on qubit 0
qc.cx(0, 1)  # CNOT: entangle qubits
```

## Progress Checklist

- [x] Understand superposition
- [x] Learn about entanglement
- [ ] Implement Shor's algorithm
- [ ] Build quantum error correction

## Further Reading

See [Qiskit Textbook](https://qiskit.org/learn)[^1] for interactive tutorials.

[^1]: IBM's open-source quantum computing framework.

---

*Press* <kbd>Space</kbd> *to toggle streaming, <kbd>r</kbd> to restart* \u{1f680}
";

const SAMPLE_MARKDOWN: &str = "\
# GitHub-Flavored Markdown

## Math & LaTeX Support

Inline math like $E = mc^2$ and $\\alpha + \\beta = \\gamma$ renders to Unicode.

Display math for complex equations:

$$\\sum_{i=1}^{n} x_i = \\frac{n(n+1)}{2}$$

$$f(x) = \\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}$$

Greek letters: $\\alpha$, $\\beta$, $\\gamma$, $\\delta$, $\\epsilon$

Operators: $\\times$, $\\div$, $\\pm$, $\\leq$, $\\geq$, $\\neq$, $\\approx$

## Task Lists

- [x] Implement markdown parser
- [x] Add LaTeX to Unicode conversion
- [x] Support task list checkboxes
- [ ] Add syntax highlighting
- [ ] Write comprehensive tests

## Admonitions

> [!NOTE]
> This is an informational note with helpful context.

> [!TIP]
> Pro tip: Use keyboard shortcuts for faster navigation.

> [!IMPORTANT]
> Critical information that users need to know.

> [!WARNING]
> Potential issues or things to be careful about.

> [!CAUTION]
> Dangerous actions that may cause problems.

## Footnotes

FrankenTUI[^1] supports GitHub-Flavored Markdown[^gfm] with
many extensions for rich terminal rendering.

[^1]: A TUI framework for Rust.
[^gfm]: GitHub-Flavored Markdown specification.

## Code Blocks

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

```python
def quicksort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    return quicksort(left) + [pivot] + quicksort(right)
```

## HTML Subset

Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to copy, <kbd>Ctrl</kbd>+<kbd>V</kbd> to paste.

Chemical formula: H<sub>2</sub>O (water), CO<sub>2</sub> (carbon dioxide)

Math notation: x<sup>2</sup> + y<sup>2</sup> = r<sup>2</sup>

## Tables

| Feature | Status | Notes |
|---------|--------|-------|
| Basic MD | ✓ | Headings, lists, emphasis |
| GFM | ✓ | Tasks, tables, footnotes |
| Math | ✓ | LaTeX → Unicode |
| Admonitions | ✓ | Note, tip, warning, etc. |

## Mermaid Diagrams

```mermaid
graph LR
    A[Input] --> B{Parse}
    B --> C[Render]
    C --> D[Display]
```

## Typography

**Bold**, *italic*, ~~strikethrough~~, `inline code`

Combined: ***bold italic***, **`bold code`**

> \"The best way to predict the future is to invent it.\"
> — Alan Kay

---

*Built with FrankenTUI \u{1f980}*
";

const WRAP_MODES: &[WrapMode] = &[
    WrapMode::None,
    WrapMode::Word,
    WrapMode::Char,
    WrapMode::WordChar,
];

const ALIGNMENTS: &[Alignment] = &[Alignment::Left, Alignment::Center, Alignment::Right];

/// Characters to advance per tick during streaming simulation.
const STREAM_CHARS_PER_TICK: usize = 3;

pub struct MarkdownRichText {
    md_scroll: u16,
    rendered_md: Text,
    wrap_index: usize,
    align_index: usize,
    // Streaming simulation state
    stream_position: usize,
    stream_paused: bool,
    stream_scroll: u16,
    md_theme: MarkdownTheme,
}

impl Default for MarkdownRichText {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRichText {
    pub fn new() -> Self {
        let md_theme = MarkdownTheme {
            h1: Style::new().fg(theme::fg::PRIMARY).bold(),
            h2: Style::new().fg(theme::accent::PRIMARY).bold(),
            h3: Style::new().fg(theme::accent::SECONDARY).bold(),
            h4: Style::new().fg(theme::accent::INFO).bold(),
            h5: Style::new().fg(theme::accent::SUCCESS).bold(),
            h6: Style::new().fg(theme::fg::SECONDARY).bold(),
            code_inline: Style::new()
                .fg(theme::accent::WARNING)
                .bg(theme::bg::SURFACE),
            code_block: Style::new().fg(theme::fg::SECONDARY).bg(theme::bg::SURFACE),
            blockquote: Style::new().fg(theme::fg::MUTED).italic(),
            link: Style::new().fg(theme::accent::LINK).underline(),
            emphasis: Style::new().italic(),
            strong: Style::new().bold(),
            strikethrough: Style::new().strikethrough(),
            list_bullet: Style::new().fg(theme::accent::PRIMARY),
            horizontal_rule: Style::new().fg(theme::fg::MUTED).dim(),
            // GFM extensions - use themed colors
            task_done: Style::new().fg(theme::accent::SUCCESS),
            task_todo: Style::new().fg(theme::accent::INFO),
            math_inline: Style::new().fg(theme::accent::SECONDARY).italic(),
            math_block: Style::new().fg(theme::accent::SECONDARY).bold(),
            footnote_ref: Style::new().fg(theme::fg::MUTED).dim(),
            footnote_def: Style::new().fg(theme::fg::SECONDARY),
            admonition_note: Style::new().fg(theme::accent::INFO).bold(),
            admonition_tip: Style::new().fg(theme::accent::SUCCESS).bold(),
            admonition_important: Style::new().fg(theme::accent::SECONDARY).bold(),
            admonition_warning: Style::new().fg(theme::accent::WARNING).bold(),
            admonition_caution: Style::new().fg(theme::accent::ERROR).bold(),
        };
        let renderer = MarkdownRenderer::new(md_theme.clone()).rule_width(36);
        let rendered_md = renderer.render(SAMPLE_MARKDOWN);

        Self {
            md_scroll: 0,
            rendered_md,
            wrap_index: 1, // Start at Word
            align_index: 0,
            // Streaming starts active
            stream_position: 0,
            stream_paused: false,
            stream_scroll: 0,
            md_theme,
        }
    }

    /// Advance the streaming simulation by one tick.
    fn tick_stream(&mut self) {
        if self.stream_paused {
            return;
        }
        let max_len = STREAMING_MARKDOWN.len();
        if self.stream_position < max_len {
            // Advance by a few characters, ensuring we land on a char boundary
            let mut new_pos = self.stream_position.saturating_add(STREAM_CHARS_PER_TICK);
            while new_pos < max_len && !STREAMING_MARKDOWN.is_char_boundary(new_pos) {
                new_pos += 1;
            }
            self.stream_position = new_pos.min(max_len);
        }
    }

    /// Get the current streaming fragment.
    fn current_stream_fragment(&self) -> &str {
        let end = self.stream_position.min(STREAMING_MARKDOWN.len());
        &STREAMING_MARKDOWN[..end]
    }

    /// Render the streaming fragment using streaming-aware rendering.
    fn render_stream_fragment(&self) -> Text {
        let fragment = self.current_stream_fragment();
        let renderer = MarkdownRenderer::new(self.md_theme.clone());
        renderer.render_streaming(fragment)
    }

    /// Check if streaming is complete.
    fn stream_complete(&self) -> bool {
        self.stream_position >= STREAMING_MARKDOWN.len()
    }

    fn current_wrap(&self) -> WrapMode {
        WRAP_MODES[self.wrap_index]
    }

    fn current_alignment(&self) -> Alignment {
        ALIGNMENTS[self.align_index]
    }

    fn wrap_label(&self) -> &'static str {
        match self.current_wrap() {
            WrapMode::None => "None",
            WrapMode::Word => "Word",
            WrapMode::Char => "Char",
            WrapMode::WordChar => "WordChar",
        }
    }

    fn alignment_label(&self) -> &'static str {
        match self.current_alignment() {
            Alignment::Left => "Left",
            Alignment::Center => "Center",
            Alignment::Right => "Right",
        }
    }

    // ---- Render panels ----

    fn render_markdown_panel(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Markdown Renderer")
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(theme::screen_accent::MARKDOWN));

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        Paragraph::new(self.rendered_md.clone())
            .wrap(WrapMode::Word)
            .scroll((self.md_scroll, 0))
            .render(inner, frame);
    }

    fn render_style_sampler(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Style Sampler")
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(theme::screen_accent::MARKDOWN));

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let styles_text = Text::from_lines([
            Line::from_spans([
                Span::styled("Bold", theme::bold()),
                Span::raw("  "),
                Span::styled("Dim", theme::dim()),
                Span::raw("  "),
                Span::styled("Italic", theme::italic()),
                Span::raw("  "),
                Span::styled("Underline", theme::underline()),
            ]),
            Line::from_spans([
                Span::styled("Strikethrough", theme::strikethrough()),
                Span::raw("  "),
                Span::styled("Reverse", theme::reverse()),
                Span::raw("  "),
                Span::styled("Blink", theme::blink_style()),
            ]),
            Line::from_spans([
                Span::styled("Dbl-Underline", theme::double_underline()),
                Span::raw("  "),
                Span::styled("Curly-Underline", theme::curly_underline()),
                Span::raw("  "),
                Span::styled("[Hidden]", theme::hidden()),
            ]),
            Line::new(),
            Line::from_spans([
                Span::styled("Error", theme::error_style()),
                Span::raw("  "),
                Span::styled("Success", theme::success()),
                Span::raw("  "),
                Span::styled("Warning", theme::warning()),
                Span::raw("  "),
                Span::styled("Link", theme::link()),
                Span::raw("  "),
                Span::styled("Code", theme::code()),
            ]),
        ]);

        Paragraph::new(styles_text).render(inner, frame);
    }

    fn render_unicode_table(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Unicode Showcase")
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(theme::screen_accent::MARKDOWN));

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let header =
            Row::new(["Text", "Type", "Cells"]).style(Style::new().fg(theme::fg::PRIMARY).bold());

        let rows = [
            Row::new(["Hello", "ASCII", "5"]),
            Row::new(["\u{4f60}\u{597d}\u{4e16}\u{754c}", "CJK", "8"]),
            Row::new(["\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}", "Hiragana", "10"]),
            Row::new(["\u{1f980}\u{1f525}\u{2728}", "Emoji", "6"]),
            Row::new(["caf\u{e9}", "Latin+accent", "4"]),
            Row::new(["\u{03b1} \u{03b2} \u{03b3} \u{03b4}", "Greek", "7"]),
            Row::new(["\u{2192} \u{2190} \u{2191} \u{2193}", "Arrows", "7"]),
            Row::new(["\u{2588}\u{2593}\u{2592}\u{2591}", "Block el.", "4"]),
        ];

        let widths = [
            Constraint::Min(12),
            Constraint::Min(12),
            Constraint::Fixed(6),
        ];

        Table::new(rows, widths)
            .header(header)
            .style(Style::new().fg(theme::fg::SECONDARY))
            .column_spacing(1)
            .render(inner, frame);
    }

    fn render_wrap_demo(&self, frame: &mut Frame, area: Rect) {
        let title = format!(
            "Wrap: {} | Align: {}",
            self.wrap_label(),
            self.alignment_label()
        );

        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title.as_str())
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(theme::screen_accent::MARKDOWN));

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let chunks = Flex::vertical()
            .constraints([Constraint::Fixed(1), Constraint::Min(1)])
            .split(inner);

        Paragraph::new("w: cycle wrap | a: cycle alignment")
            .style(theme::muted())
            .render(chunks[0], frame);

        let demo_text = "The quick brown fox jumps over the lazy dog. \
             Supercalifragilisticexpialidocious is quite a long word \
             that tests character-level wrapping behavior. \
             \u{4f60}\u{597d}\u{4e16}\u{754c} contains CJK characters \
             that are double-width. \u{1f980} Ferris says hello!";

        Paragraph::new(demo_text)
            .wrap(self.current_wrap())
            .alignment(self.current_alignment())
            .style(Style::new().fg(theme::fg::PRIMARY))
            .render(chunks[1], frame);
    }

    fn render_streaming_panel(&self, frame: &mut Frame, area: Rect) {
        // Build title with streaming status
        let progress_pct =
            (self.stream_position as f64 / STREAMING_MARKDOWN.len() as f64 * 100.0) as u8;
        let status = if self.stream_complete() {
            "Complete".to_string()
        } else if self.stream_paused {
            format!("Paused ({progress_pct}%)")
        } else {
            format!("Streaming... {progress_pct}%")
        };

        let title = format!("LLM Streaming Simulation | {status}");

        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(title.as_str())
            .title_alignment(Alignment::Center)
            .style(Style::new().fg(theme::screen_accent::MARKDOWN));

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        // Split into content area and detection info
        let chunks = Flex::vertical()
            .constraints([Constraint::Min(5), Constraint::Fixed(3)])
            .split(inner);

        // Render the streaming markdown fragment
        let stream_text = self.render_stream_fragment();
        Paragraph::new(stream_text)
            .wrap(WrapMode::Word)
            .scroll((self.stream_scroll, 0))
            .render(chunks[0], frame);

        // Detection status panel
        let fragment = self.current_stream_fragment();
        let detection = is_likely_markdown(fragment);
        let det_line1 = format!(
            "Detection: {} indicators | {}",
            detection.indicators,
            if detection.is_confident() {
                "Confident"
            } else if detection.is_likely() {
                "Likely"
            } else {
                "Uncertain"
            }
        );
        let det_line2 = format!(
            "Confidence: {:.0}% | Chars: {}/{}",
            detection.confidence() * 100.0,
            self.stream_position,
            STREAMING_MARKDOWN.len()
        );
        let det_line3 = "Space: play/pause | r: restart | ↑↓: scroll stream";

        let detection_text = Text::from_lines([
            Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled(det_line1, theme::muted()),
            ]),
            Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled(det_line2, theme::muted()),
            ]),
            Line::from_spans([
                Span::styled("  ", Style::new()),
                Span::styled(det_line3, Style::new().fg(theme::accent::INFO).dim()),
            ]),
        ]);

        Paragraph::new(detection_text).render(chunks[1], frame);
    }
}

impl Screen for MarkdownRichText {
    type Message = Event;

    fn update(&mut self, event: &Event) -> Cmd<Self::Message> {
        // Handle tick events for streaming
        if let Event::Tick = event {
            self.tick_stream();
        }

        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match code {
                // Markdown panel scrolling
                KeyCode::Up => {
                    self.md_scroll = self.md_scroll.saturating_sub(1);
                }
                KeyCode::Down => {
                    self.md_scroll = self.md_scroll.saturating_add(1);
                }
                KeyCode::PageUp => {
                    self.md_scroll = self.md_scroll.saturating_sub(10);
                }
                KeyCode::PageDown => {
                    self.md_scroll = self.md_scroll.saturating_add(10);
                }
                KeyCode::Home => {
                    self.md_scroll = 0;
                }
                // Wrap/alignment controls
                KeyCode::Char('w') => {
                    self.wrap_index = (self.wrap_index + 1) % WRAP_MODES.len();
                }
                KeyCode::Char('a') => {
                    self.align_index = (self.align_index + 1) % ALIGNMENTS.len();
                }
                // Streaming controls
                KeyCode::Char(' ') => {
                    self.stream_paused = !self.stream_paused;
                }
                KeyCode::Char('r') => {
                    // Reset streaming
                    self.stream_position = 0;
                    self.stream_paused = false;
                    self.stream_scroll = 0;
                }
                KeyCode::Char('[') => {
                    // Scroll stream panel up
                    self.stream_scroll = self.stream_scroll.saturating_sub(1);
                }
                KeyCode::Char(']') => {
                    // Scroll stream panel down
                    self.stream_scroll = self.stream_scroll.saturating_add(1);
                }
                _ => {}
            }
        }
        Cmd::None
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        // Main layout: three columns - left markdown, center streaming, right panels
        let cols = Flex::horizontal()
            .constraints([
                Constraint::Percentage(35.0),
                Constraint::Percentage(35.0),
                Constraint::Percentage(30.0),
            ])
            .split(area);

        // Left: Full GFM markdown demo
        self.render_markdown_panel(frame, cols[0]);

        // Center: Streaming simulation
        self.render_streaming_panel(frame, cols[1]);

        // Right: Auxiliary panels
        let right_rows = Flex::vertical()
            .constraints([
                Constraint::Fixed(8),
                Constraint::Fixed(1),
                Constraint::Min(6),
            ])
            .split(cols[2]);

        self.render_style_sampler(frame, right_rows[0]);

        Rule::new()
            .style(Style::new().fg(theme::fg::MUTED).dim())
            .render(right_rows[1], frame);

        self.render_wrap_demo(frame, right_rows[2]);
    }

    fn keybindings(&self) -> Vec<HelpEntry> {
        vec![
            HelpEntry {
                key: "\u{2191}/\u{2193}",
                action: "Scroll markdown",
            },
            HelpEntry {
                key: "[/]",
                action: "Scroll stream",
            },
            HelpEntry {
                key: "Space",
                action: "Play/pause stream",
            },
            HelpEntry {
                key: "r",
                action: "Restart stream",
            },
            HelpEntry {
                key: "w/a",
                action: "Wrap/align mode",
            },
        ]
    }

    fn title(&self) -> &'static str {
        "Markdown and Rich Text"
    }

    fn tab_label(&self) -> &'static str {
        "Markdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: ftui_core::event::Modifiers::empty(),
            kind: KeyEventKind::Press,
        })
    }

    #[test]
    fn initial_state() {
        let screen = MarkdownRichText::new();
        assert_eq!(screen.md_scroll, 0);
        assert_eq!(screen.title(), "Markdown and Rich Text");
        assert_eq!(screen.tab_label(), "Markdown");
    }

    #[test]
    fn markdown_renders_headings() {
        let screen = MarkdownRichText::new();
        let plain: String = screen
            .rendered_md
            .lines()
            .iter()
            .map(|l| l.to_plain_text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(plain.contains("GitHub-Flavored Markdown"));
        assert!(plain.contains("Math & LaTeX Support"));
        assert!(plain.contains("Task Lists"));
    }

    #[test]
    fn markdown_renders_code_block() {
        let screen = MarkdownRichText::new();
        let plain: String = screen
            .rendered_md
            .lines()
            .iter()
            .map(|l| l.to_plain_text())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(plain.contains("fn fibonacci"));
        assert!(plain.contains("def quicksort"));
    }

    #[test]
    fn markdown_renders_task_lists() {
        let screen = MarkdownRichText::new();
        let plain: String = screen
            .rendered_md
            .lines()
            .iter()
            .map(|l| l.to_plain_text())
            .collect::<Vec<_>>()
            .join("\n");
        // Task list items should have checkbox markers
        assert!(plain.contains("Implement markdown parser"));
        assert!(plain.contains("Add syntax highlighting"));
    }

    #[test]
    fn scroll_navigation() {
        let mut screen = MarkdownRichText::new();
        screen.update(&press(KeyCode::Down));
        assert_eq!(screen.md_scroll, 1);
        screen.update(&press(KeyCode::Down));
        assert_eq!(screen.md_scroll, 2);
        screen.update(&press(KeyCode::Up));
        assert_eq!(screen.md_scroll, 1);
        screen.update(&press(KeyCode::Home));
        assert_eq!(screen.md_scroll, 0);
        screen.update(&press(KeyCode::Up));
        assert_eq!(screen.md_scroll, 0);
    }

    #[test]
    fn page_scroll() {
        let mut screen = MarkdownRichText::new();
        screen.update(&press(KeyCode::PageDown));
        assert_eq!(screen.md_scroll, 10);
        screen.update(&press(KeyCode::PageUp));
        assert_eq!(screen.md_scroll, 0);
    }

    #[test]
    fn wrap_mode_cycles() {
        let mut screen = MarkdownRichText::new();
        assert_eq!(screen.wrap_label(), "Word");
        screen.update(&press(KeyCode::Char('w')));
        assert_eq!(screen.wrap_label(), "Char");
        screen.update(&press(KeyCode::Char('w')));
        assert_eq!(screen.wrap_label(), "WordChar");
        screen.update(&press(KeyCode::Char('w')));
        assert_eq!(screen.wrap_label(), "None");
        screen.update(&press(KeyCode::Char('w')));
        assert_eq!(screen.wrap_label(), "Word");
    }

    #[test]
    fn alignment_cycles() {
        let mut screen = MarkdownRichText::new();
        assert_eq!(screen.alignment_label(), "Left");
        screen.update(&press(KeyCode::Char('a')));
        assert_eq!(screen.alignment_label(), "Center");
        screen.update(&press(KeyCode::Char('a')));
        assert_eq!(screen.alignment_label(), "Right");
        screen.update(&press(KeyCode::Char('a')));
        assert_eq!(screen.alignment_label(), "Left");
    }

    #[test]
    fn keybindings_non_empty() {
        let screen = MarkdownRichText::new();
        assert!(!screen.keybindings().is_empty());
    }

    #[test]
    fn style_flags_all_represented() {
        let styles = [
            theme::bold(),
            theme::dim(),
            theme::italic(),
            theme::underline(),
            theme::strikethrough(),
            theme::reverse(),
            theme::blink_style(),
            theme::double_underline(),
            theme::curly_underline(),
        ];
        for style in &styles {
            assert_ne!(*style, Style::default());
        }
    }
}

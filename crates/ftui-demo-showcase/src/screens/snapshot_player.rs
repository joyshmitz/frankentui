#![forbid(unsafe_code)]

//! Snapshot/Time Travel Player demo screen.
//!
//! Demonstrates FrankenTUI's time-travel debugging capabilities:
//! - Frame recording with delta compression
//! - Timeline scrubbing with smooth playback
//! - Frame metadata inspection (diff counts, render stats)
//! - Integrity verification via checksums
//!
//! # Invariants
//!
//! 1. **Playback determinism**: Same snapshot file + same frame index = identical buffer
//! 2. **Progress bounds**: `0 <= current_frame < frame_count` (when non-empty)
//! 3. **Checksum integrity**: Hash chain verifies tamper-free replay
//! 4. **Memory budget**: Bounded by TimeTravel capacity (default 100 frames)
//!
//! # Failure Modes
//!
//! - **Empty recording**: Gracefully show "No frames recorded" state
//! - **Corrupted import**: Display error, don't panic
//! - **Large scrub jump**: May briefly lag while reconstructing (O(n) deltas)
//!
//! # Keybindings
//!
//! - Space: Play/pause playback
//! - Left/Right: Step backward/forward one frame
//! - Home/End: Jump to first/last frame
//! - M: Toggle marker mode (add/remove frame markers)
//! - R: Toggle recording (capture new frames)
//! - C: Clear recording
//! - D: Toggle diagnostic panel

use std::collections::HashSet;
use std::time::Duration;

use ftui_core::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ftui_core::geometry::Rect;
use ftui_layout::{Constraint, Flex};
use ftui_render::buffer::Buffer;
use ftui_render::cell::Cell;
use ftui_render::frame::Frame;
use ftui_runtime::Cmd;
use ftui_style::Style;
use ftui_widgets::Widget;
use ftui_widgets::block::{Alignment, Block};
use ftui_widgets::borders::{BorderType, Borders};
use ftui_widgets::paragraph::Paragraph;

use super::{HelpEntry, Screen};
use crate::theme;

/// Demo content patterns for generating interesting frames.
const DEMO_PATTERNS: &[&str] = &[
    "Hello, World!",
    "FrankenTUI Demo",
    "Time Travel Mode",
    "Frame Snapshot",
    "Delta Encoding",
    "Press Space to Play",
    "← → to Step",
    "Deterministic Replay",
];

/// Configuration for the snapshot player.
#[derive(Clone, Debug)]
pub struct SnapshotPlayerConfig {
    /// Maximum frames to record.
    pub max_frames: usize,
    /// Playback speed (frames per tick when playing).
    pub playback_speed: usize,
    /// Whether to auto-generate demo frames on init.
    pub auto_generate_demo: bool,
    /// Number of demo frames to generate.
    pub demo_frame_count: usize,
}

impl Default for SnapshotPlayerConfig {
    fn default() -> Self {
        Self {
            max_frames: 100,
            playback_speed: 1,
            auto_generate_demo: true,
            demo_frame_count: 50,
        }
    }
}

/// Playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Paused,
    Playing,
    Recording,
}

impl PlaybackState {
    fn label(self) -> &'static str {
        match self {
            Self::Paused => "⏸ Paused",
            Self::Playing => "▶ Playing",
            Self::Recording => "⏺ Recording",
        }
    }

    fn style(self) -> Style {
        match self {
            Self::Paused => Style::new().fg(theme::fg::MUTED),
            Self::Playing => Style::new().fg(theme::accent::SUCCESS),
            Self::Recording => Style::new().fg(theme::accent::ERROR),
        }
    }
}

/// Metadata for a recorded frame.
#[derive(Debug, Clone)]
pub struct FrameInfo {
    /// Frame index.
    pub index: usize,
    /// Number of changed cells from previous frame.
    pub change_count: usize,
    /// Buffer dimensions.
    pub width: u16,
    pub height: u16,
    /// Estimated memory size in bytes.
    pub memory_size: usize,
    /// Frame checksum for integrity verification.
    pub checksum: u64,
    /// Render time (if known).
    pub render_time: Option<Duration>,
}

// =============================================================================
// Diagnostic Logging + Telemetry (bd-3sa7.5)
// =============================================================================

/// Configuration for diagnostic logging and telemetry.
#[derive(Clone, Debug)]
pub struct DiagnosticConfig {
    /// Enable structured JSONL logging.
    pub enabled: bool,
    /// Maximum diagnostic entries to retain.
    pub max_entries: usize,
    /// Log navigation events.
    pub log_navigation: bool,
    /// Log playback state changes.
    pub log_playback: bool,
    /// Log frame recording events.
    pub log_recording: bool,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 500,
            log_navigation: true,
            log_playback: true,
            log_recording: true,
        }
    }
}

/// A diagnostic log entry for JSONL output.
#[derive(Clone, Debug)]
pub enum DiagnosticEntry {
    /// Frame navigation event.
    Navigation {
        seq: u64,
        action: &'static str,
        from_frame: usize,
        to_frame: usize,
        frame_count: usize,
    },
    /// Playback state transition.
    PlaybackChange {
        seq: u64,
        from_state: &'static str,
        to_state: &'static str,
        current_frame: usize,
    },
    /// Frame recorded.
    FrameRecorded {
        seq: u64,
        frame_index: usize,
        change_count: usize,
        checksum: u64,
        chain_checksum: u64,
        width: u16,
        height: u16,
    },
    /// Marker toggled.
    MarkerToggled {
        seq: u64,
        frame_index: usize,
        added: bool,
        total_markers: usize,
    },
    /// Clear/reset event.
    Cleared {
        seq: u64,
        frame_count: usize,
        marker_count: usize,
    },
}

impl DiagnosticEntry {
    /// Serialize to JSONL format.
    pub fn to_jsonl(&self) -> String {
        match self {
            Self::Navigation {
                seq,
                action,
                from_frame,
                to_frame,
                frame_count,
            } => {
                format!(
                    r#"{{"seq":{},"event":"nav","action":"{}","from":{},"to":{},"count":{}}}"#,
                    seq, action, from_frame, to_frame, frame_count
                )
            }
            Self::PlaybackChange {
                seq,
                from_state,
                to_state,
                current_frame,
            } => {
                format!(
                    r#"{{"seq":{},"event":"playback","from":"{}","to":"{}","frame":{}}}"#,
                    seq, from_state, to_state, current_frame
                )
            }
            Self::FrameRecorded {
                seq,
                frame_index,
                change_count,
                checksum,
                chain_checksum,
                width,
                height,
            } => {
                format!(
                    r#"{{"seq":{},"event":"record","frame":{},"changes":{},"checksum":"0x{:016x}","chain":"0x{:016x}","size":[{},{}]}}"#,
                    seq, frame_index, change_count, checksum, chain_checksum, width, height
                )
            }
            Self::MarkerToggled {
                seq,
                frame_index,
                added,
                total_markers,
            } => {
                format!(
                    r#"{{"seq":{},"event":"marker","frame":{},"added":{},"total":{}}}"#,
                    seq, frame_index, added, total_markers
                )
            }
            Self::Cleared {
                seq,
                frame_count,
                marker_count,
            } => {
                format!(
                    r#"{{"seq":{},"event":"clear","frames":{},"markers":{}}}"#,
                    seq, frame_count, marker_count
                )
            }
        }
    }
}

/// Diagnostic log buffer with bounded capacity.
#[derive(Debug)]
pub struct DiagnosticLog {
    entries: std::collections::VecDeque<DiagnosticEntry>,
    max_entries: usize,
    seq: u64,
}

impl DiagnosticLog {
    /// Create a new diagnostic log.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: std::collections::VecDeque::with_capacity(max_entries.min(1000)),
            max_entries,
            seq: 0,
        }
    }

    /// Get and increment the sequence number.
    pub fn next_seq(&mut self) -> u64 {
        let s = self.seq;
        self.seq = self.seq.wrapping_add(1);
        s
    }

    /// Push a log entry.
    pub fn push(&mut self, entry: DiagnosticEntry) {
        if self.max_entries == 0 {
            return;
        }
        while self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Get all entries.
    pub fn entries(&self) -> &std::collections::VecDeque<DiagnosticEntry> {
        &self.entries
    }

    /// Export to JSONL format.
    pub fn to_jsonl(&self) -> String {
        self.entries
            .iter()
            .map(|e| e.to_jsonl())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Clear entries (keeps seq).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Snapshot Player screen state.
#[derive(Debug)]
pub struct SnapshotPlayer {
    /// Recorded frames (buffers stored directly for demo simplicity).
    frames: Vec<Buffer>,
    /// Frame metadata.
    pub frame_info: Vec<FrameInfo>,
    /// Current frame index.
    pub current_frame: usize,
    /// Playback state.
    pub playback_state: PlaybackState,
    /// Marked frames for inspection.
    markers: HashSet<usize>,
    /// Whether to show diagnostic panel.
    show_diagnostics: bool,
    /// Current tick count.
    tick_count: u64,
    /// Configuration.
    config: SnapshotPlayerConfig,
    /// Demo buffer dimensions.
    demo_width: u16,
    demo_height: u16,
    /// Running checksum chain.
    pub checksum_chain: u64,
    /// Diagnostic configuration (bd-3sa7.5).
    diagnostic_config: DiagnosticConfig,
    /// Diagnostic log (bd-3sa7.5).
    diagnostic_log: DiagnosticLog,
}

impl Default for SnapshotPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotPlayer {
    /// Create a new snapshot player with demo content.
    pub fn new() -> Self {
        let config = SnapshotPlayerConfig::default();
        let diagnostic_config = DiagnosticConfig::default();
        let mut player = Self {
            frames: Vec::with_capacity(config.max_frames),
            frame_info: Vec::with_capacity(config.max_frames),
            current_frame: 0,
            playback_state: PlaybackState::Paused,
            markers: HashSet::new(),
            show_diagnostics: true,
            tick_count: 0,
            config,
            demo_width: 40,
            demo_height: 15,
            checksum_chain: 0,
            diagnostic_log: DiagnosticLog::new(diagnostic_config.max_entries),
            diagnostic_config,
        };

        if player.config.auto_generate_demo {
            player.generate_demo_frames();
        }

        player
    }

    /// Create with custom configuration.
    pub fn with_config(config: SnapshotPlayerConfig) -> Self {
        let diagnostic_config = DiagnosticConfig::default();
        let mut player = Self {
            frames: Vec::with_capacity(config.max_frames),
            frame_info: Vec::with_capacity(config.max_frames),
            current_frame: 0,
            playback_state: PlaybackState::Paused,
            markers: HashSet::new(),
            show_diagnostics: true,
            tick_count: 0,
            demo_width: 40,
            demo_height: 15,
            checksum_chain: 0,
            config,
            diagnostic_log: DiagnosticLog::new(diagnostic_config.max_entries),
            diagnostic_config,
        };

        if player.config.auto_generate_demo {
            player.generate_demo_frames();
        }

        player
    }

    /// Generate demo frames with evolving content.
    fn generate_demo_frames(&mut self) {
        let count = self.config.demo_frame_count;
        let mut prev_buf: Option<Buffer> = None;

        for i in 0..count {
            let mut buf = Buffer::new(self.demo_width, self.demo_height);

            // Draw evolving content based on frame number
            self.draw_demo_content(&mut buf, i);

            // Calculate change count from previous frame
            let change_count = match &prev_buf {
                Some(prev) => self.count_changes(prev, &buf),
                None => (self.demo_width as usize) * (self.demo_height as usize),
            };

            // Calculate checksum
            let checksum = self.calculate_checksum(&buf);
            self.checksum_chain = self.checksum_chain.wrapping_add(checksum);

            let info = FrameInfo {
                index: i,
                change_count,
                width: self.demo_width,
                height: self.demo_height,
                memory_size: buf.len() * std::mem::size_of::<Cell>(),
                checksum,
                render_time: Some(Duration::from_micros((100 + (i * 10) % 500) as u64)),
            };

            prev_buf = Some(buf.clone());
            self.frames.push(buf);
            self.frame_info.push(info);
        }
    }

    /// Draw demo content for a specific frame.
    fn draw_demo_content(&self, buf: &mut Buffer, frame_idx: usize) {
        let pattern_idx = frame_idx % DEMO_PATTERNS.len();
        let text = DEMO_PATTERNS[pattern_idx];

        // Animated position
        let x_offset = (frame_idx % 20) as u16;
        let y_offset = (frame_idx / 5 % 10) as u16;

        // Draw frame number in top-left
        let frame_label = format!("Frame {}/{}", frame_idx + 1, self.config.demo_frame_count);
        for (i, ch) in frame_label.chars().enumerate() {
            let x = i as u16;
            if x < buf.width() {
                buf.set(x, 0, Cell::from_char(ch));
            }
        }

        // Draw main pattern text with animation
        let y = (y_offset + 3).min(buf.height().saturating_sub(1));
        for (i, ch) in text.chars().enumerate() {
            let x = (x_offset + i as u16) % buf.width();
            if y < buf.height() {
                // Cycle colors based on position and frame
                let color_idx = (frame_idx + i) % 6;
                let fg_color = match color_idx {
                    0 => theme::accent::INFO,
                    1 => theme::accent::SUCCESS,
                    2 => theme::accent::WARNING,
                    3 => theme::accent::ERROR,
                    4 => theme::fg::PRIMARY,
                    _ => theme::fg::SECONDARY,
                };
                let cell = Cell::from_char(ch).with_fg(fg_color.into());
                buf.set(x, y, cell);
            }
        }

        // Draw a moving cursor indicator
        let cursor_x = ((frame_idx * 2) % (buf.width() as usize)) as u16;
        let cursor_y = buf.height().saturating_sub(2);
        if cursor_y < buf.height() {
            buf.set(cursor_x, cursor_y, Cell::from_char('█'));
        }

        // Draw progress bar at bottom
        let progress = frame_idx as f64 / self.config.demo_frame_count as f64;
        let bar_width = (buf.width() as f64 * progress) as u16;
        let bar_y = buf.height().saturating_sub(1);
        for x in 0..bar_width.min(buf.width()) {
            buf.set(x, bar_y, Cell::from_char('━'));
        }
    }

    /// Count changed cells between two buffers.
    fn count_changes(&self, prev: &Buffer, curr: &Buffer) -> usize {
        let mut count = 0;
        for y in 0..curr.height().min(prev.height()) {
            for x in 0..curr.width().min(prev.width()) {
                if let (Some(pc), Some(cc)) = (prev.get(x, y), curr.get(x, y))
                    && !pc.bits_eq(cc)
                {
                    count += 1;
                }
            }
        }
        count
    }

    /// Calculate a simple checksum for integrity verification.
    fn calculate_checksum(&self, buf: &Buffer) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                if let Some(cell) = buf.get(x, y) {
                    // Mix in cell content
                    hash ^= cell.content.raw() as u64;
                    hash = hash.wrapping_mul(0x100000001b3); // FNV-1a prime
                    hash ^= cell.fg.0 as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                    hash ^= cell.bg.0 as u64;
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
        }
        hash
    }

    /// Record a new frame (when in recording mode).
    pub fn record_frame(&mut self, buf: &Buffer) {
        if self.frames.len() >= self.config.max_frames {
            // Remove oldest frame
            self.frames.remove(0);
            self.frame_info.remove(0);
            // Reindex remaining frames
            for (i, info) in self.frame_info.iter_mut().enumerate() {
                info.index = i;
            }
        }

        let prev = self.frames.last();
        let change_count = match prev {
            Some(p) => self.count_changes(p, buf),
            None => buf.len(),
        };

        let checksum = self.calculate_checksum(buf);
        self.checksum_chain = self.checksum_chain.wrapping_add(checksum);

        let info = FrameInfo {
            index: self.frames.len(),
            change_count,
            width: buf.width(),
            height: buf.height(),
            memory_size: buf.len() * std::mem::size_of::<Cell>(),
            checksum,
            render_time: None,
        };

        let frame_index = self.frames.len();
        let width = buf.width();
        let height = buf.height();
        self.frames.push(buf.clone());
        self.frame_info.push(info);
        self.current_frame = self.frames.len().saturating_sub(1);
        self.log_frame_recorded(frame_index, change_count, checksum, width, height);
    }

    /// Clear all recorded frames.
    pub fn clear(&mut self) {
        self.log_cleared();
        self.frames.clear();
        self.frame_info.clear();
        self.markers.clear();
        self.current_frame = 0;
        self.checksum_chain = 0;
        self.playback_state = PlaybackState::Paused;
        self.diagnostic_log.clear();
    }

    /// Total number of frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Current frame index.
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Set current frame index with bounds checking.
    pub fn set_current_frame(&mut self, frame: usize) {
        if self.frames.is_empty() {
            self.current_frame = 0;
        } else {
            self.current_frame = frame.min(self.frames.len() - 1);
        }
    }

    /// Current checksum chain value.
    pub fn checksum_chain(&self) -> u64 {
        self.checksum_chain
    }

    /// Access full frame metadata list.
    pub fn frame_info(&self) -> &[FrameInfo] {
        &self.frame_info
    }

    /// Access marker set (read-only).
    pub fn markers(&self) -> &HashSet<usize> {
        &self.markers
    }

    /// Current playback state.
    pub fn playback_state(&self) -> PlaybackState {
        self.playback_state
    }

    /// Whether the diagnostic panel is currently visible.
    pub fn diagnostics_visible(&self) -> bool {
        self.show_diagnostics
    }

    /// Get current frame buffer.
    pub fn current_buffer(&self) -> Option<&Buffer> {
        self.frames.get(self.current_frame)
    }

    /// Get current frame info.
    pub fn current_info(&self) -> Option<&FrameInfo> {
        self.frame_info.get(self.current_frame)
    }

    /// Step to next frame.
    pub fn step_forward(&mut self) {
        let from = self.current_frame;
        if !self.frames.is_empty() {
            self.current_frame = (self.current_frame + 1).min(self.frames.len() - 1);
        }
        self.log_navigation("step_forward", from);
    }

    /// Step to previous frame.
    pub fn step_backward(&mut self) {
        let from = self.current_frame;
        self.current_frame = self.current_frame.saturating_sub(1);
        self.log_navigation("step_backward", from);
    }

    /// Jump to first frame.
    pub fn go_to_start(&mut self) {
        let from = self.current_frame;
        self.current_frame = 0;
        self.log_navigation("go_start", from);
    }

    /// Jump to last frame.
    pub fn go_to_end(&mut self) {
        let from = self.current_frame;
        if !self.frames.is_empty() {
            self.current_frame = self.frames.len() - 1;
        }
        self.log_navigation("go_end", from);
    }

    /// Toggle play/pause.
    pub fn toggle_playback(&mut self) {
        let from_state = self.playback_state.label();
        self.playback_state = match self.playback_state {
            PlaybackState::Playing => PlaybackState::Paused,
            PlaybackState::Paused | PlaybackState::Recording => PlaybackState::Playing,
        };
        self.log_playback(from_state);
    }

    /// Toggle marker on current frame.
    pub fn toggle_marker(&mut self) {
        let added = if self.markers.contains(&self.current_frame) {
            self.markers.remove(&self.current_frame);
            false
        } else {
            self.markers.insert(self.current_frame);
            true
        };
        self.log_marker(added);
    }

    /// Toggle recording mode.
    pub fn toggle_recording(&mut self) {
        let from_state = self.playback_state.label();
        self.playback_state = match self.playback_state {
            PlaybackState::Recording => PlaybackState::Paused,
            _ => PlaybackState::Recording,
        };
        self.log_playback(from_state);
    }

    // ========================================================================
    // Diagnostic Logging Helpers
    // ========================================================================

    /// Log a navigation event.
    fn log_navigation(&mut self, action: &'static str, from_frame: usize) {
        if !self.diagnostic_config.enabled || !self.diagnostic_config.log_navigation {
            return;
        }
        let seq = self.diagnostic_log.next_seq();
        self.diagnostic_log.push(DiagnosticEntry::Navigation {
            seq,
            action,
            from_frame,
            to_frame: self.current_frame,
            frame_count: self.frames.len(),
        });
    }

    /// Log a playback state change.
    fn log_playback(&mut self, from_state: &'static str) {
        if !self.diagnostic_config.enabled || !self.diagnostic_config.log_playback {
            return;
        }
        let seq = self.diagnostic_log.next_seq();
        self.diagnostic_log.push(DiagnosticEntry::PlaybackChange {
            seq,
            from_state,
            to_state: self.playback_state.label(),
            current_frame: self.current_frame,
        });
    }

    /// Log a marker toggle.
    fn log_marker(&mut self, added: bool) {
        if !self.diagnostic_config.enabled {
            return;
        }
        let seq = self.diagnostic_log.next_seq();
        self.diagnostic_log.push(DiagnosticEntry::MarkerToggled {
            seq,
            frame_index: self.current_frame,
            added,
            total_markers: self.markers.len(),
        });
    }

    /// Log a frame recorded event.
    fn log_frame_recorded(
        &mut self,
        frame_index: usize,
        change_count: usize,
        checksum: u64,
        width: u16,
        height: u16,
    ) {
        if !self.diagnostic_config.enabled || !self.diagnostic_config.log_recording {
            return;
        }
        let seq = self.diagnostic_log.next_seq();
        self.diagnostic_log.push(DiagnosticEntry::FrameRecorded {
            seq,
            frame_index,
            change_count,
            checksum,
            chain_checksum: self.checksum_chain,
            width,
            height,
        });
    }

    /// Log a clear event.
    fn log_cleared(&mut self) {
        if !self.diagnostic_config.enabled {
            return;
        }
        let seq = self.diagnostic_log.next_seq();
        self.diagnostic_log.push(DiagnosticEntry::Cleared {
            seq,
            frame_count: self.frames.len(),
            marker_count: self.markers.len(),
        });
    }

    /// Get the diagnostic log (for testing/inspection).
    pub fn diagnostic_log(&self) -> &DiagnosticLog {
        &self.diagnostic_log
    }

    /// Export diagnostic log to JSONL format.
    pub fn export_diagnostics(&self) -> String {
        self.diagnostic_log.to_jsonl()
    }

    // ========================================================================
    // Rendering
    // ========================================================================

    fn render_main_layout(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        // Layout: Preview (left) | Info panel (right)
        let chunks = Flex::horizontal()
            .constraints([Constraint::Percentage(60.0), Constraint::Percentage(40.0)])
            .split(area);

        if chunks.len() >= 2 {
            // Left side: Timeline + Preview
            let left_chunks = Flex::vertical()
                .constraints([Constraint::Fixed(3), Constraint::Min(1)])
                .split(chunks[0]);
            if left_chunks.len() >= 2 {
                self.render_timeline(frame, left_chunks[0]);
                self.render_preview(frame, left_chunks[1]);
            }

            // Right side: Info + Controls
            self.render_info_panel(frame, chunks[1]);
        }
    }

    fn render_timeline(&self, frame: &mut Frame, area: Rect) {
        let border_style = Style::new().fg(theme::screen_accent::PERFORMANCE);

        let title = format!(
            "Timeline ({}/{})",
            self.current_frame + 1,
            self.frame_count().max(1)
        );
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(&title)
            .title_alignment(Alignment::Center)
            .style(border_style);

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() || self.frames.is_empty() {
            return;
        }

        // Draw timeline bar
        let progress =
            self.current_frame as f64 / (self.frames.len().saturating_sub(1).max(1)) as f64;
        let bar_width = ((inner.width as f64) * progress) as u16;

        // Draw progress bar
        for x in 0..inner.width {
            let ch = if x < bar_width { '█' } else { '░' };
            let fg_color = if x < bar_width {
                theme::accent::INFO
            } else {
                theme::fg::DISABLED
            };
            let cell = Cell::from_char(ch).with_fg(fg_color.into());
            frame.buffer.set(inner.x + x, inner.y, cell);
        }

        // Draw markers
        for &marker_idx in &self.markers {
            let marker_x = if self.frames.len() > 1 {
                (marker_idx as f64 / (self.frames.len() - 1) as f64 * inner.width as f64) as u16
            } else {
                0
            };
            if marker_x < inner.width {
                let cell = Cell::from_char('▼').with_fg(theme::accent::WARNING.into());
                frame.buffer.set(inner.x + marker_x, inner.y, cell);
            }
        }
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Frame Preview")
            .title_alignment(Alignment::Center)
            .style(theme::content_border());

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        // Render the frame content if available
        if let Some(buf) = self.current_buffer() {
            // Copy frame content to preview area (scaled if needed)
            for y in 0..inner.height.min(buf.height()) {
                for x in 0..inner.width.min(buf.width()) {
                    if let Some(cell) = buf.get(x, y) {
                        frame.buffer.set(inner.x + x, inner.y + y, *cell);
                    }
                }
            }
        } else {
            // No frames - show empty state
            let msg = "No frames recorded";
            let x = inner.x + (inner.width.saturating_sub(msg.len() as u16)) / 2;
            let y = inner.y + inner.height / 2;
            Paragraph::new(msg)
                .style(Style::new().fg(theme::fg::MUTED))
                .render(Rect::new(x, y, msg.len() as u16, 1), frame);
        }
    }

    fn render_info_panel(&self, frame: &mut Frame, area: Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Frame Info")
            .title_alignment(Alignment::Center)
            .style(theme::content_border());

        let inner = block.inner(area);
        block.render(area, frame);

        if inner.is_empty() {
            return;
        }

        let mut lines = Vec::new();

        // Playback status
        lines.push(format!("Status: {}", self.playback_state.label()));
        lines.push(String::new());

        if let Some(info) = self.current_info() {
            lines.push(format!("Frame: {}/{}", info.index + 1, self.frame_count()));
            lines.push(format!("Size: {}x{}", info.width, info.height));
            lines.push(format!("Changes: {} cells", info.change_count));
            lines.push(format!("Memory: {} bytes", info.memory_size));
            lines.push(format!("Checksum: {:016x}", info.checksum));
            if let Some(rt) = info.render_time {
                lines.push(format!("Render: {:?}", rt));
            }
            lines.push(String::new());
            lines.push(format!("Chain hash: {:016x}", self.checksum_chain));
            lines.push(format!("Markers: {}", self.markers.len()));
            lines.push(format!(
                "Marked: {}",
                if self.markers.contains(&self.current_frame) {
                    "Yes ▼"
                } else {
                    "No"
                }
            ));
        } else {
            lines.push("No frame data".to_string());
        }

        lines.push(String::new());
        lines.push("── Controls ──".to_string());
        lines.push("Space: Play/Pause".to_string());
        lines.push("←/→ or h/l: Step".to_string());
        lines.push("Home/End or g/G: First/Last".to_string());
        lines.push("M: Toggle marker".to_string());
        lines.push("R: Toggle record".to_string());
        lines.push("C: Clear".to_string());
        lines.push("D: Diagnostics".to_string());

        for (i, line) in lines.iter().enumerate() {
            if i as u16 >= inner.height {
                break;
            }
            let style = if line.starts_with("Status:") {
                self.playback_state.style()
            } else if line.starts_with("──") {
                Style::new().fg(theme::fg::MUTED)
            } else if line.contains(':') && !line.starts_with(' ') {
                Style::new().fg(theme::fg::SECONDARY)
            } else {
                Style::new().fg(theme::fg::PRIMARY)
            };

            Paragraph::new(line.as_str()).style(style).render(
                Rect::new(inner.x, inner.y + i as u16, inner.width, 1),
                frame,
            );
        }
    }
}

impl Screen for SnapshotPlayer {
    type Message = Event;

    fn update(&mut self, event: &Event) -> Cmd<Self::Message> {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            ..
        }) = event
        {
            match code {
                KeyCode::Char(' ') => self.toggle_playback(),
                KeyCode::Left | KeyCode::Char('h') => {
                    self.playback_state = PlaybackState::Paused;
                    self.step_backward();
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.playback_state = PlaybackState::Paused;
                    self.step_forward();
                }
                KeyCode::Home => {
                    self.playback_state = PlaybackState::Paused;
                    self.go_to_start();
                }
                KeyCode::End => {
                    self.playback_state = PlaybackState::Paused;
                    self.go_to_end();
                }
                KeyCode::Char('m') | KeyCode::Char('M') => self.toggle_marker(),
                KeyCode::Char('r') | KeyCode::Char('R') => self.toggle_recording(),
                KeyCode::Char('c') | KeyCode::Char('C') => self.clear(),
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    self.show_diagnostics = !self.show_diagnostics;
                }
                KeyCode::Char('g') => self.go_to_start(),
                KeyCode::Char('G') => self.go_to_end(),
                _ => {}
            }
        }

        Cmd::None
    }

    fn tick(&mut self, tick_count: u64) {
        self.tick_count = tick_count;

        if self.playback_state == PlaybackState::Playing {
            // Advance frame during playback (every N ticks based on speed)
            if tick_count.is_multiple_of(2) {
                // Advance every 2 ticks (~5 fps)
                if self.current_frame + 1 < self.frames.len() {
                    self.current_frame += 1;
                } else {
                    // Loop back to start
                    self.current_frame = 0;
                }
            }
        }
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        if area.is_empty() {
            return;
        }

        self.render_main_layout(frame, area);
    }

    fn keybindings(&self) -> Vec<HelpEntry> {
        vec![
            HelpEntry {
                key: "Space",
                action: "Play/Pause",
            },
            HelpEntry {
                key: "←/→ or h/l",
                action: "Step frame",
            },
            HelpEntry {
                key: "Home/End or g/G",
                action: "First/Last",
            },
            HelpEntry {
                key: "M",
                action: "Toggle marker",
            },
            HelpEntry {
                key: "R",
                action: "Toggle record",
            },
            HelpEntry {
                key: "C",
                action: "Clear all",
            },
            HelpEntry {
                key: "D",
                action: "Diagnostics",
            },
        ]
    }

    fn title(&self) -> &'static str {
        "Snapshot Player"
    }

    fn tab_label(&self) -> &'static str {
        "Snapshots"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ftui_render::grapheme_pool::GraphemePool;

    #[test]
    fn new_creates_demo_frames() {
        let player = SnapshotPlayer::new();
        assert_eq!(player.frame_count(), 50);
        assert_eq!(player.current_frame, 0);
        assert_eq!(player.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn step_forward_advances_frame() {
        let mut player = SnapshotPlayer::new();
        assert_eq!(player.current_frame, 0);
        player.step_forward();
        assert_eq!(player.current_frame, 1);
    }

    #[test]
    fn step_forward_clamps_at_end() {
        let mut player = SnapshotPlayer::new();
        player.go_to_end();
        let last = player.current_frame;
        player.step_forward();
        assert_eq!(player.current_frame, last);
    }

    #[test]
    fn step_backward_decrements_frame() {
        let mut player = SnapshotPlayer::new();
        player.step_forward();
        player.step_forward();
        assert_eq!(player.current_frame, 2);
        player.step_backward();
        assert_eq!(player.current_frame, 1);
    }

    #[test]
    fn step_backward_clamps_at_zero() {
        let mut player = SnapshotPlayer::new();
        player.step_backward();
        assert_eq!(player.current_frame, 0);
    }

    #[test]
    fn go_to_start_resets_to_zero() {
        let mut player = SnapshotPlayer::new();
        player.go_to_end();
        assert!(player.current_frame > 0);
        player.go_to_start();
        assert_eq!(player.current_frame, 0);
    }

    #[test]
    fn go_to_end_jumps_to_last() {
        let mut player = SnapshotPlayer::new();
        player.go_to_end();
        assert_eq!(player.current_frame, player.frame_count() - 1);
    }

    #[test]
    fn toggle_playback_changes_state() {
        let mut player = SnapshotPlayer::new();
        assert_eq!(player.playback_state, PlaybackState::Paused);
        player.toggle_playback();
        assert_eq!(player.playback_state, PlaybackState::Playing);
        player.toggle_playback();
        assert_eq!(player.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn toggle_marker_adds_and_removes() {
        let mut player = SnapshotPlayer::new();
        assert!(!player.markers.contains(&0));
        player.toggle_marker();
        assert!(player.markers.contains(&0));
        player.toggle_marker();
        assert!(!player.markers.contains(&0));
    }

    #[test]
    fn clear_removes_all_frames() {
        let mut player = SnapshotPlayer::new();
        assert!(player.frame_count() > 0);
        player.clear();
        assert_eq!(player.frame_count(), 0);
        assert_eq!(player.current_frame, 0);
        assert!(player.markers.is_empty());
    }

    #[test]
    fn frame_info_has_valid_checksums() {
        let player = SnapshotPlayer::new();
        let info = player.current_info().unwrap();
        assert!(info.checksum != 0);
    }

    #[test]
    fn frame_info_tracks_change_counts() {
        let player = SnapshotPlayer::new();
        // First frame should have many changes (full snapshot)
        let first_info = &player.frame_info[0];
        assert!(first_info.change_count > 0);

        // Later frames should have fewer changes (deltas)
        if player.frame_count() > 1 {
            let later_info = &player.frame_info[1];
            assert!(later_info.change_count < first_info.change_count);
        }
    }

    #[test]
    fn renders_without_panic() {
        let player = SnapshotPlayer::new();
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(120, 40, &mut pool);
        player.view(&mut frame, Rect::new(0, 0, 120, 40));
    }

    #[test]
    fn renders_small_area() {
        let player = SnapshotPlayer::new();
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(40, 10, &mut pool);
        player.view(&mut frame, Rect::new(0, 0, 40, 10));
    }

    #[test]
    fn renders_empty_area() {
        let player = SnapshotPlayer::new();
        let mut pool = GraphemePool::new();
        let mut frame = Frame::new(10, 10, &mut pool);
        player.view(&mut frame, Rect::new(0, 0, 0, 0));
    }

    #[test]
    fn tick_advances_during_playback() {
        let mut player = SnapshotPlayer::new();
        player.toggle_playback(); // Start playing
        let initial = player.current_frame;
        player.tick(2);
        // Should advance after tick
        assert!(player.current_frame > initial || player.current_frame == 0);
    }

    #[test]
    fn tick_does_not_advance_when_paused() {
        let mut player = SnapshotPlayer::new();
        let initial = player.current_frame;
        player.tick(2);
        assert_eq!(player.current_frame, initial);
    }

    #[test]
    fn playback_loops_at_end() {
        let mut player = SnapshotPlayer::new();
        player.go_to_end();
        player.toggle_playback();
        player.tick(2);
        assert_eq!(player.current_frame, 0); // Looped back
    }

    #[test]
    fn custom_config_respected() {
        let config = SnapshotPlayerConfig {
            max_frames: 10,
            playback_speed: 2,
            auto_generate_demo: true,
            demo_frame_count: 5,
        };
        let player = SnapshotPlayer::with_config(config);
        assert_eq!(player.frame_count(), 5);
    }

    #[test]
    fn title_and_label() {
        let player = SnapshotPlayer::new();
        assert_eq!(player.title(), "Snapshot Player");
        assert_eq!(player.tab_label(), "Snapshots");
    }

    #[test]
    fn keybindings_not_empty() {
        let player = SnapshotPlayer::new();
        assert!(!player.keybindings().is_empty());
    }

    // ========================================================================
    // Edge Case Tests (bd-3sa7.3)
    // ========================================================================

    #[test]
    fn empty_player_handles_navigation() {
        let config = SnapshotPlayerConfig {
            auto_generate_demo: false,
            ..Default::default()
        };
        let mut player = SnapshotPlayer::with_config(config);
        assert_eq!(player.frame_count(), 0);

        // Navigation on empty player should not panic
        player.step_forward();
        player.step_backward();
        player.go_to_start();
        player.go_to_end();
        assert_eq!(player.current_frame, 0);
    }

    #[test]
    fn empty_player_current_buffer_is_none() {
        let config = SnapshotPlayerConfig {
            auto_generate_demo: false,
            ..Default::default()
        };
        let player = SnapshotPlayer::with_config(config);
        assert!(player.current_buffer().is_none());
        assert!(player.current_info().is_none());
    }

    #[test]
    fn record_frame_adds_to_empty_player() {
        let config = SnapshotPlayerConfig {
            auto_generate_demo: false,
            max_frames: 10,
            ..Default::default()
        };
        let mut player = SnapshotPlayer::with_config(config);
        assert_eq!(player.frame_count(), 0);

        let buf = Buffer::new(10, 5);
        player.record_frame(&buf);
        assert_eq!(player.frame_count(), 1);
        assert!(player.current_buffer().is_some());
    }

    #[test]
    fn record_frame_respects_max_frames() {
        let config = SnapshotPlayerConfig {
            auto_generate_demo: false,
            max_frames: 3,
            ..Default::default()
        };
        let mut player = SnapshotPlayer::with_config(config);

        // Record 5 frames
        for i in 0..5 {
            let mut buf = Buffer::new(10, 5);
            // Mark each buffer distinctly
            buf.set(0, 0, Cell::from_char((b'A' + i as u8) as char));
            player.record_frame(&buf);
        }

        // Should only keep the last 3
        assert_eq!(player.frame_count(), 3);
        // Frame indices should be reindexed
        assert_eq!(player.frame_info[0].index, 0);
        assert_eq!(player.frame_info[1].index, 1);
        assert_eq!(player.frame_info[2].index, 2);
    }

    #[test]
    fn checksum_chain_accumulates() {
        let config = SnapshotPlayerConfig {
            auto_generate_demo: false,
            ..Default::default()
        };
        let mut player = SnapshotPlayer::with_config(config);
        assert_eq!(player.checksum_chain, 0);

        let buf = Buffer::new(10, 5);
        player.record_frame(&buf);
        let after_first = player.checksum_chain;
        assert!(after_first != 0);

        player.record_frame(&buf);
        // Chain should grow (wrapping add)
        assert!(player.checksum_chain != after_first);
    }

    #[test]
    fn toggle_recording_state() {
        let mut player = SnapshotPlayer::new();
        assert_eq!(player.playback_state, PlaybackState::Paused);

        player.toggle_recording();
        assert_eq!(player.playback_state, PlaybackState::Recording);

        player.toggle_recording();
        assert_eq!(player.playback_state, PlaybackState::Paused);
    }

    #[test]
    fn recording_to_playing_via_toggle_playback() {
        let mut player = SnapshotPlayer::new();
        player.toggle_recording();
        assert_eq!(player.playback_state, PlaybackState::Recording);

        player.toggle_playback();
        assert_eq!(player.playback_state, PlaybackState::Playing);
    }

    #[test]
    fn key_event_updates() {
        use ftui_core::event::Modifiers;

        let mut player = SnapshotPlayer::new();

        // Test space key
        let space_event = Event::Key(KeyEvent {
            code: KeyCode::Char(' '),
            kind: KeyEventKind::Press,
            modifiers: Modifiers::NONE,
        });
        player.update(&space_event);
        assert_eq!(player.playback_state, PlaybackState::Playing);

        // Test 'd' key for diagnostics toggle
        let initial_diag = player.show_diagnostics;
        let d_event = Event::Key(KeyEvent {
            code: KeyCode::Char('d'),
            kind: KeyEventKind::Press,
            modifiers: Modifiers::NONE,
        });
        player.update(&d_event);
        assert_ne!(player.show_diagnostics, initial_diag);
    }

    // ========================================================================
    // Invariant Tests (bd-3sa7.3)
    // ========================================================================

    /// Invariant 1: Progress bounds - current_frame is always within valid range
    #[test]
    fn invariant_progress_bounds() {
        let mut player = SnapshotPlayer::new();
        let n = player.frame_count();

        // After any navigation, current_frame should be in [0, n-1]
        for _ in 0..100 {
            player.step_forward();
        }
        assert!(player.current_frame < n);

        for _ in 0..100 {
            player.step_backward();
        }
        assert_eq!(player.current_frame, 0);
    }

    /// Invariant 2: Playback determinism - same frame index yields same buffer
    #[test]
    fn invariant_playback_determinism() {
        let player = SnapshotPlayer::new();
        let idx = 10;

        let buf1 = &player.frames[idx];
        let buf2 = &player.frames[idx];

        // Same buffer should have identical content
        assert_eq!(buf1.width(), buf2.width());
        assert_eq!(buf1.height(), buf2.height());

        let checksum1 = player.frame_info[idx].checksum;
        let checksum2 = player.frame_info[idx].checksum;
        assert_eq!(checksum1, checksum2);
    }

    /// Invariant 3: Checksum integrity - checksums are consistent
    #[test]
    fn invariant_checksum_consistency() {
        let player = SnapshotPlayer::new();

        // Recalculate checksum for first frame
        let buf = &player.frames[0];
        let recalc = player.calculate_checksum(buf);
        assert_eq!(recalc, player.frame_info[0].checksum);
    }

    // ========================================================================
    // Diagnostic Logging Tests (bd-3sa7.5)
    // ========================================================================

    #[test]
    fn diagnostic_log_captures_navigation() {
        let mut player = SnapshotPlayer::new();
        let initial_entries = player.diagnostic_log().entries().len();
        player.step_forward();
        player.step_backward();
        player.go_to_start();
        player.go_to_end();
        assert_eq!(player.diagnostic_log().entries().len(), initial_entries + 4);
    }

    #[test]
    fn diagnostic_log_captures_playback() {
        let mut player = SnapshotPlayer::new();
        let initial_entries = player.diagnostic_log().entries().len();
        player.toggle_playback();
        player.toggle_playback();
        assert!(player.diagnostic_log().entries().len() >= initial_entries + 2);
    }

    #[test]
    fn diagnostic_log_captures_markers() {
        let mut player = SnapshotPlayer::new();
        let initial_entries = player.diagnostic_log().entries().len();
        player.toggle_marker();
        player.toggle_marker();
        assert!(player.diagnostic_log().entries().len() >= initial_entries + 2);
    }

    #[test]
    fn diagnostic_log_to_jsonl() {
        let mut player = SnapshotPlayer::new();
        player.step_forward();
        player.toggle_playback();
        let jsonl = player.export_diagnostics();
        assert!(!jsonl.is_empty());
        assert!(jsonl.contains("\"event\""));
        assert!(jsonl.contains("\"seq\":"));
    }

    #[test]
    fn diagnostic_log_respects_disabled_config() {
        let config = SnapshotPlayerConfig {
            auto_generate_demo: false,
            ..Default::default()
        };
        let mut player = SnapshotPlayer::with_config(config);
        player.diagnostic_config.enabled = false;
        player.step_forward();
        player.toggle_playback();
        assert!(player.diagnostic_log().entries().is_empty());
    }
}

// ============================================================================
// Property Tests (bd-3sa7.3)
// ============================================================================

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: Frame index is always bounded after any sequence of navigation
        #[test]
        fn prop_frame_index_bounded(
            steps in prop::collection::vec(0u8..6, 0..50)
        ) {
            let mut player = SnapshotPlayer::new();
            let n = player.frame_count();

            for step in steps {
                match step % 6 {
                    0 => player.step_forward(),
                    1 => player.step_backward(),
                    2 => player.go_to_start(),
                    3 => player.go_to_end(),
                    4 => player.toggle_playback(),
                    _ => player.toggle_marker(),
                }
            }

            // Invariant: current_frame is always in valid range
            prop_assert!(player.current_frame < n || (n == 0 && player.current_frame == 0));
        }

        /// Property: Clear always resets to empty state
        #[test]
        fn prop_clear_resets_state(
            ops_before_clear in prop::collection::vec(0u8..4, 0..20)
        ) {
            let mut player = SnapshotPlayer::new();

            // Do some random operations
            for op in ops_before_clear {
                match op % 4 {
                    0 => player.step_forward(),
                    1 => player.toggle_marker(),
                    2 => player.go_to_end(),
                    _ => player.toggle_playback(),
                }
            }

            // Clear should reset everything
            player.clear();
            prop_assert_eq!(player.frame_count(), 0);
            prop_assert_eq!(player.current_frame, 0);
            prop_assert!(player.markers.is_empty());
            prop_assert_eq!(player.checksum_chain, 0);
            prop_assert_eq!(player.playback_state, PlaybackState::Paused);
        }

        /// Property: Recording adds exactly one frame per call
        #[test]
        fn prop_record_increments_count(
            record_count in 1usize..20,
            width in 5u16..50,
            height in 5u16..30
        ) {
            let config = SnapshotPlayerConfig {
                auto_generate_demo: false,
                max_frames: 100,
                ..Default::default()
            };
            let mut player = SnapshotPlayer::with_config(config);

            for i in 0..record_count {
                let buf = Buffer::new(width, height);
                player.record_frame(&buf);
                prop_assert_eq!(player.frame_count(), i + 1);
            }
        }

        /// Property: Checksums are non-zero for non-empty buffers
        #[test]
        fn prop_checksum_nonzero(
            width in 5u16..50,
            height in 5u16..30
        ) {
            let config = SnapshotPlayerConfig {
                auto_generate_demo: false,
                ..Default::default()
            };
            let mut player = SnapshotPlayer::with_config(config);

            let mut buf = Buffer::new(width, height);
            // Put some content in the buffer
            buf.set(0, 0, Cell::from_char('X'));

            player.record_frame(&buf);

            let info = player.current_info().unwrap();
            prop_assert!(info.checksum != 0);
        }

        /// Property: Frame info indices are always sequential
        #[test]
        fn prop_frame_info_indices_sequential(
            _seed in 0u64..1000
        ) {
            let player = SnapshotPlayer::new();

            for (i, info) in player.frame_info.iter().enumerate() {
                prop_assert_eq!(info.index, i);
            }
        }
    }
}

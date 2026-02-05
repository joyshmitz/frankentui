#![forbid(unsafe_code)]

//! Render kernel: cells, buffers, diffs, and ANSI presentation.
//!
//! # Role in FrankenTUI
//! `ftui-render` is the deterministic rendering engine. It turns a logical
//! `Frame` into a `Buffer`, computes diffs, and emits minimal ANSI output via
//! the `Presenter`.
//!
//! # Primary responsibilities
//! - **Cell/Buffer**: 2D grid with fixed-size cells and scissor/opacity stacks.
//! - **BufferDiff**: efficient change detection between frames.
//! - **Presenter**: stateful ANSI emitter with cursor/mode tracking.
//! - **Frame**: rendering surface used by widgets and application views.
//!
//! # How it fits in the system
//! `ftui-runtime` calls your model's `view()` to render into a `Frame`. That
//! frame becomes a `Buffer`, which is diffed and presented to the terminal via
//! `TerminalWriter`. This crate is the kernel of FrankenTUI's flicker-free,
//! deterministic output guarantees.

pub mod alloc_budget;
pub mod ansi;
pub mod budget;
pub mod buffer;
pub mod cell;
pub mod counting_writer;
pub mod diff;
pub mod diff_strategy;
pub mod drawing;
pub mod frame;
pub mod grapheme_pool;
pub mod headless;
pub mod link_registry;
pub mod presenter;
pub mod sanitize;
pub mod spatial_hit_index;
pub mod terminal_model;

mod text_width {
    #[inline]
    pub(crate) fn grapheme_width(grapheme: &str) -> usize {
        ftui_core::text_width::grapheme_width(grapheme)
    }

    #[inline]
    pub(crate) fn char_width(ch: char) -> usize {
        ftui_core::text_width::char_width(ch)
    }

    #[inline]
    pub(crate) fn display_width(text: &str) -> usize {
        ftui_core::text_width::display_width(text)
    }
}

pub(crate) use text_width::{char_width, display_width, grapheme_width};

#[cfg(test)]
mod tests {
    use super::{display_width, grapheme_width};

    #[test]
    fn display_width_matches_expected_samples() {
        // Avoid CJK samples to keep results independent of locale/CJK width flags.
        let samples = [
            ("hello", 5usize),
            ("😀", 2usize),
            ("👩‍💻", 2usize),
            ("🇺🇸", 2usize),
            ("❤️", 2usize),
            ("⌨️", 2usize),
            ("⚠️", 2usize),
            ("⭐", 2usize),
            ("A😀B", 4usize),
            ("ok ✅", 5usize),
        ];
        for (sample, expected) in samples {
            assert_eq!(
                display_width(sample),
                expected,
                "display width mismatch for {sample:?}"
            );
        }
    }

    #[test]
    fn grapheme_width_matches_expected_samples() {
        let samples = [
            ("a", 1usize),
            ("😀", 2usize),
            ("👩‍💻", 2usize),
            ("🇺🇸", 2usize),
            ("👍🏽", 2usize),
            ("❤️", 2usize),
            ("⌨️", 2usize),
            ("⚠️", 2usize),
            ("⭐", 2usize),
        ];
        for (grapheme, expected) in samples {
            assert_eq!(
                grapheme_width(grapheme),
                expected,
                "grapheme width mismatch for {grapheme:?}"
            );
        }
    }
}

#![forbid(unsafe_code)]
#![allow(dead_code)]

//! FrankenTUI Demo Showcase library.
//!
//! This module exposes the demo application internals so that integration tests
//! can construct screens, render them, and assert snapshots.

pub mod app;
pub mod chrome;
pub mod cli;
pub mod data;
pub mod screens;
pub mod theme;
pub mod tour;

/// Debug logging macro for visual render diagnostics (bd-3vbf.31).
///
/// Only emits to stderr when `debug-render` feature is enabled.
/// Usage: `debug_render!("dashboard", "layout={layout:?}, area={area:?}");`
#[cfg(feature = "debug-render")]
#[macro_export]
macro_rules! debug_render {
    ($component:expr, $($arg:tt)*) => {
        eprintln!("[debug-render][{}] {}", $component, format_args!($($arg)*));
    };
}

#[cfg(not(feature = "debug-render"))]
#[macro_export]
macro_rules! debug_render {
    ($component:expr, $($arg:tt)*) => {};
}

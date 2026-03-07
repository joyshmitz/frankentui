#![forbid(unsafe_code)]

//! Reusable diagnostic logging and telemetry substrate.
//!
//! This module provides the shared infrastructure for JSONL diagnostic
//! logging and telemetry hooks, extracted from the common patterns in
//! [`crate::inspector`] and the demo showcase's `mouse_playground`.
//!
//! # Design
//!
//! The core types are generic over the entry type, so each consumer
//! defines its own `DiagnosticEntry` / `DiagnosticEventKind` while
//! reusing the log, dispatch, and checksum infrastructure.
//!
//! # Key Types
//!
//! - [`DiagnosticRecord`] — trait for entries that can be serialized to JSONL
//! - [`DiagnosticLog`] — bounded in-memory log with optional stderr mirroring
//! - [`TelemetryCallback`] — type alias for observer callbacks
//! - [`fnv1a_hash`] — FNV-1a checksum utility for determinism verification
//!
//! # Example
//!
//! ```ignore
//! use ftui_widgets::diagnostics::{DiagnosticLog, DiagnosticRecord};
//!
//! #[derive(Debug, Clone)]
//! struct MyEntry { kind: &'static str, data: u64 }
//!
//! impl DiagnosticRecord for MyEntry {
//!     fn to_jsonl(&self) -> String {
//!         format!("{{\"kind\":\"{}\",\"data\":{}}}", self.kind, self.data)
//!     }
//! }
//!
//! let mut log = DiagnosticLog::<MyEntry>::new();
//! log.record(MyEntry { kind: "test", data: 42 });
//! assert_eq!(log.entries().len(), 1);
//! ```

use std::fmt;
use std::io::Write;

// =============================================================================
// DiagnosticRecord trait
// =============================================================================

/// Trait for diagnostic entries that can be serialized to JSONL.
///
/// Consumers define their own entry structs with domain-specific fields
/// and implement this trait to plug into [`DiagnosticLog`].
pub trait DiagnosticRecord: fmt::Debug + Clone {
    /// Format this entry as a single JSONL line (no trailing newline).
    fn to_jsonl(&self) -> String;
}

// =============================================================================
// DiagnosticLog<E>
// =============================================================================

/// Bounded in-memory diagnostic log with optional stderr mirroring.
///
/// Generic over the entry type `E` so different subsystems can use
/// their own entry structs while sharing the log infrastructure.
#[derive(Debug)]
pub struct DiagnosticLog<E: DiagnosticRecord> {
    /// Collected entries.
    entries: Vec<E>,
    /// Maximum entries to keep (0 = unlimited).
    max_entries: usize,
    /// Whether to also write to stderr.
    write_stderr: bool,
}

impl<E: DiagnosticRecord> Default for DiagnosticLog<E> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: DiagnosticRecord> DiagnosticLog<E> {
    /// Create a new diagnostic log with a default capacity of 10 000 entries.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 10_000,
            write_stderr: false,
        }
    }

    /// Enable stderr mirroring — each recorded entry is also written
    /// to stderr as a JSONL line.
    #[must_use]
    pub fn with_stderr(mut self) -> Self {
        self.write_stderr = true;
        self
    }

    /// Set the maximum number of entries to keep. When the log is full,
    /// the oldest entry is evicted. Pass `0` for unlimited.
    #[must_use]
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Record a diagnostic entry.
    pub fn record(&mut self, entry: E) {
        if self.write_stderr {
            let _ = writeln!(std::io::stderr(), "{}", entry.to_jsonl());
        }

        if self.max_entries > 0 && self.entries.len() >= self.max_entries {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    /// Get all entries.
    pub fn entries(&self) -> &[E] {
        &self.entries
    }

    /// Get entries matching a predicate.
    pub fn entries_matching(&self, predicate: impl Fn(&E) -> bool) -> Vec<&E> {
        self.entries.iter().filter(|e| predicate(e)).collect()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Export all entries as a JSONL string (newline-separated).
    pub fn to_jsonl(&self) -> String {
        self.entries
            .iter()
            .map(DiagnosticRecord::to_jsonl)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Number of recorded entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// =============================================================================
// TelemetryCallback type alias
// =============================================================================

/// Callback type for telemetry hooks.
///
/// Generic over the entry type so each subsystem can observe its own
/// domain-specific entries.
pub type TelemetryCallback<E> = Box<dyn Fn(&E) + Send + Sync>;

// =============================================================================
// FNV-1a checksum utility
// =============================================================================

/// Compute an FNV-1a 64-bit hash of the given byte slice.
///
/// This is the same algorithm used by both `inspector` and
/// `mouse_playground` for determinism verification checksums.
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// =============================================================================
// Helpers for environment-flag-based diagnostics
// =============================================================================

/// Check an environment variable as a boolean diagnostic flag.
///
/// Returns `true` if the variable is set to `"true"` (case-insensitive).
pub fn env_flag_enabled(var_name: &str) -> bool {
    std::env::var(var_name)
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestEntry {
        kind: &'static str,
        value: u64,
    }

    impl DiagnosticRecord for TestEntry {
        fn to_jsonl(&self) -> String {
            format!("{{\"kind\":\"{}\",\"value\":{}}}", self.kind, self.value)
        }
    }

    #[test]
    fn log_records_and_retrieves() {
        let mut log = DiagnosticLog::<TestEntry>::new();
        log.record(TestEntry {
            kind: "a",
            value: 1,
        });
        log.record(TestEntry {
            kind: "b",
            value: 2,
        });
        assert_eq!(log.len(), 2);
        assert_eq!(log.entries()[0].value, 1);
        assert_eq!(log.entries()[1].value, 2);
    }

    #[test]
    fn log_evicts_oldest_when_full() {
        let mut log = DiagnosticLog::<TestEntry>::new().with_max_entries(2);
        log.record(TestEntry {
            kind: "a",
            value: 1,
        });
        log.record(TestEntry {
            kind: "b",
            value: 2,
        });
        log.record(TestEntry {
            kind: "c",
            value: 3,
        });
        assert_eq!(log.len(), 2);
        assert_eq!(log.entries()[0].value, 2);
        assert_eq!(log.entries()[1].value, 3);
    }

    #[test]
    fn log_clear() {
        let mut log = DiagnosticLog::<TestEntry>::new();
        log.record(TestEntry {
            kind: "a",
            value: 1,
        });
        assert!(!log.is_empty());
        log.clear();
        assert!(log.is_empty());
        assert_eq!(log.len(), 0);
    }

    #[test]
    fn log_to_jsonl() {
        let mut log = DiagnosticLog::<TestEntry>::new();
        log.record(TestEntry {
            kind: "x",
            value: 10,
        });
        log.record(TestEntry {
            kind: "y",
            value: 20,
        });
        let output = log.to_jsonl();
        assert!(output.contains("\"kind\":\"x\""));
        assert!(output.contains("\"kind\":\"y\""));
        assert!(output.contains('\n'));
    }

    #[test]
    fn log_entries_matching() {
        let mut log = DiagnosticLog::<TestEntry>::new();
        log.record(TestEntry {
            kind: "a",
            value: 1,
        });
        log.record(TestEntry {
            kind: "b",
            value: 2,
        });
        log.record(TestEntry {
            kind: "a",
            value: 3,
        });
        let matches = log.entries_matching(|e| e.kind == "a");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn fnv1a_hash_deterministic() {
        let h1 = fnv1a_hash(b"hello world");
        let h2 = fnv1a_hash(b"hello world");
        assert_eq!(h1, h2);
        assert_ne!(h1, fnv1a_hash(b"hello worlD"));
    }

    #[test]
    fn fnv1a_hash_empty() {
        let h = fnv1a_hash(b"");
        assert_eq!(h, 0xcbf29ce484222325); // FNV offset basis
    }

    #[test]
    fn env_flag_enabled_false_when_unset() {
        // Use a unique variable name unlikely to be set
        assert!(!env_flag_enabled("FTUI_TEST_DIAGNOSTICS_NEVER_SET_12345"));
    }

    #[test]
    fn default_log_has_correct_capacity() {
        let log = DiagnosticLog::<TestEntry>::new();
        assert_eq!(log.max_entries, 10_000);
        assert!(!log.write_stderr);
    }
}

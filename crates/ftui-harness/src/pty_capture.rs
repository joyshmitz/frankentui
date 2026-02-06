#![forbid(unsafe_code)]

//! Harness helper for routing subprocess PTY output through the one-writer path.
//!
//! This module is feature-gated behind `pty-capture` and is intended for
//! harness-style integrations that need to run tools while keeping inline
//! mode stable (sanitize-by-default + log routing).

use ftui_extras::pty_capture::{PtyCapture, PtyCaptureConfig};
use ftui_runtime::log_sink::LogSink;
use ftui_runtime::terminal_writer::TerminalWriter;
use portable_pty::CommandBuilder;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

/// Run a command in a PTY and stream its output through a [`LogSink`].
///
/// This preserves sanitize-by-default (via `LogSink`) and keeps terminal
/// output within the one-writer path.
pub fn run_command_with_pty<W: Write>(
    writer: &mut TerminalWriter<W>,
    cmd: CommandBuilder,
    config: PtyCaptureConfig,
) -> io::Result<portable_pty::ExitStatus> {
    let mut capture = PtyCapture::spawn(config, cmd)?;
    let mut sink = LogSink::new(writer);

    loop {
        let drained = capture.drain_to_log_sink(&mut sink)?;
        if drained == 0 {
            if capture.is_eof() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    sink.flush()?;
    capture.wait()
}

#[cfg(all(test, feature = "pty-capture", unix))]
mod tests {
    use super::*;
    use crate::determinism::{JsonValue, TestJsonlLogger};
    use crate::golden::compute_text_checksum;
    use ftui_core::terminal_capabilities::TerminalCapabilities;
    use ftui_runtime::terminal_writer::{ScreenMode, UiAnchor};
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};

    fn create_writer() -> TerminalWriter<Vec<u8>> {
        TerminalWriter::new(
            Vec::new(),
            ScreenMode::Inline { ui_height: 5 },
            UiAnchor::Bottom,
            TerminalCapabilities::basic(),
        )
    }

    fn create_writer_with_mode(mode: ScreenMode) -> TerminalWriter<Vec<u8>> {
        TerminalWriter::new(
            Vec::new(),
            mode,
            UiAnchor::Bottom,
            TerminalCapabilities::basic(),
        )
    }

    fn strip_ansi(input: &[u8]) -> String {
        let mut out = Vec::with_capacity(input.len());
        let mut i = 0;
        while i < input.len() {
            if input[i] == 0x1b {
                if i + 1 >= input.len() {
                    break;
                }
                let next = input[i + 1];
                if next == b'[' {
                    i += 2;
                    while i < input.len() {
                        let byte = input[i];
                        i += 1;
                        if (0x40..=0x7e).contains(&byte) {
                            break;
                        }
                    }
                    continue;
                }
                if next == b']' {
                    i += 2;
                    while i < input.len() {
                        if input[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if input[i] == 0x1b && i + 1 < input.len() && input[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
                i += 2;
                continue;
            }
            out.push(input[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).to_string()
    }

    fn normalize_output(raw: &[u8]) -> String {
        let stripped = strip_ansi(raw);
        stripped
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .trim()
            .to_string()
    }

    fn logger() -> &'static TestJsonlLogger {
        static LOGGER: OnceLock<TestJsonlLogger> = OnceLock::new();
        LOGGER.get_or_init(|| {
            let mut logger = TestJsonlLogger::new("pty_capture_harness", 4242);
            logger.add_context_str("suite", "pty_capture_harness");
            logger
        })
    }

    fn capture_text(mode: ScreenMode, cols: u16, rows: u16, command: &str) -> (String, String) {
        let mut writer = create_writer_with_mode(mode);
        let mut cmd = CommandBuilder::new("sh");
        cmd.args(["-c", command]);
        let config = PtyCaptureConfig::default().with_size(cols, rows);
        let _ = run_command_with_pty(&mut writer, cmd, config);

        let output = writer.into_inner().unwrap_or_default();
        let text = normalize_output(&output);
        let checksum = compute_text_checksum(&text);
        (text, checksum)
    }

    #[test]
    fn harness_pty_capture_sanitizes() {
        let mut writer = create_writer();
        let mut cmd = CommandBuilder::new("sh");
        cmd.args(["-c", "printf 'ok \\033[31mred\\033[0m\\n'"]);

        let _ = run_command_with_pty(&mut writer, cmd, PtyCaptureConfig::default());

        let output = writer.into_inner().unwrap();
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("ok red"));
        assert!(!output_str.contains("\x1b[31m"));
    }

    #[test]
    fn harness_pty_capture_inline_and_altscreen_consistent() {
        let cols = 40;
        let rows = 10;
        let command = "printf 'ok \\033[31mred\\033[0m\\n'";

        let (inline_text, inline_checksum) =
            capture_text(ScreenMode::Inline { ui_height: 5 }, cols, rows, command);
        let (alt_text, alt_checksum) = capture_text(ScreenMode::AltScreen, cols, rows, command);

        assert!(
            inline_text.contains("ok red"),
            "inline output missing expected text: {inline_text:?}"
        );
        assert!(
            alt_text.is_empty(),
            "expected altscreen capture to be empty (TerminalWriter drops log writes): {alt_text:?}"
        );

        logger().log_env();
        logger().log(
            "pty_capture",
            &[
                ("mode", JsonValue::str("inline")),
                ("cols", JsonValue::u64(cols as u64)),
                ("rows", JsonValue::u64(rows as u64)),
                ("checksum", JsonValue::str(inline_checksum)),
            ],
        );
        logger().log(
            "pty_capture",
            &[
                ("mode", JsonValue::str("altscreen")),
                ("cols", JsonValue::u64(cols as u64)),
                ("rows", JsonValue::u64(rows as u64)),
                ("checksum", JsonValue::str(alt_checksum)),
            ],
        );
    }

    #[test]
    fn harness_pty_capture_replay_deterministic() {
        let cols = 48;
        let rows = 12;
        let command = "printf 'alpha\\n'; printf 'beta\\n'; printf 'gamma\\n'";

        let (text1, checksum1) =
            capture_text(ScreenMode::Inline { ui_height: 5 }, cols, rows, command);
        let (text2, checksum2) =
            capture_text(ScreenMode::Inline { ui_height: 5 }, cols, rows, command);

        assert_eq!(
            text1, text2,
            "expected deterministic PTY capture text across replays"
        );
        assert_eq!(
            checksum1, checksum2,
            "expected deterministic PTY capture checksum across replays"
        );

        logger().log_env();
        logger().log(
            "pty_capture_replay",
            &[
                ("cols", JsonValue::u64(cols as u64)),
                ("rows", JsonValue::u64(rows as u64)),
                ("checksum", JsonValue::str(checksum1)),
            ],
        );
    }

    #[test]
    fn harness_pty_capture_respects_size() {
        let cols = 52;
        let rows = 14;
        let (text, checksum) =
            capture_text(ScreenMode::Inline { ui_height: 5 }, cols, rows, "stty size");
        let mut parts = text.split_whitespace();
        let row_str = parts.next().unwrap_or_default();
        let col_str = parts.next().unwrap_or_default();
        assert_eq!(row_str, rows.to_string(), "row count mismatch in stty size");
        assert_eq!(col_str, cols.to_string(), "col count mismatch in stty size");

        logger().log(
            "pty_capture_size",
            &[
                ("cols", JsonValue::u64(cols as u64)),
                ("rows", JsonValue::u64(rows as u64)),
                ("checksum", JsonValue::str(checksum)),
            ],
        );
    }

    #[test]
    fn harness_pty_capture_partial_reads_preserve_order() {
        let mut cmd = CommandBuilder::new("sh");
        cmd.args(["-c", "printf 'alpha'; sleep 0.05; printf 'beta'"]);

        let mut capture =
            PtyCapture::spawn(PtyCaptureConfig::default(), cmd).expect("spawn PTY capture");

        let mut collected = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let chunk = capture
                .read_available_with_timeout(Duration::from_millis(50))
                .expect("read output");
            if !chunk.is_empty() {
                collected.extend_from_slice(&chunk);
            }
            if capture.is_eof() {
                break;
            }
        }

        let _ = capture.wait();
        for _ in 0..5 {
            if capture.is_eof() {
                break;
            }
            let chunk = capture
                .read_available_with_timeout(Duration::from_millis(50))
                .expect("drain output");
            if !chunk.is_empty() {
                collected.extend_from_slice(&chunk);
            }
        }

        let text = normalize_output(&collected);
        let alpha_pos = text.find("alpha").expect("missing alpha output");
        let beta_pos = text.find("beta").expect("missing beta output");
        assert!(
            alpha_pos < beta_pos,
            "expected alpha before beta in output: {text:?}"
        );

        logger().log(
            "pty_capture_partial",
            &[("checksum", JsonValue::str(compute_text_checksum(&text)))],
        );
    }
}

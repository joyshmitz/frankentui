#![forbid(unsafe_code)]

//! Glyph capability policy (Unicode/ASCII, emoji, and width calibration).
//!
//! This module centralizes glyph decisions so rendering and demos can
//! consistently choose Unicode vs ASCII, emoji usage, and CJK width policy.
//! Decisions are deterministic given environment variables and a terminal
//! capability profile.

use crate::terminal_capabilities::{TerminalCapabilities, TerminalProfile};
use crate::text_width;

/// Environment variable to override glyph mode (`unicode` or `ascii`).
const ENV_GLYPH_MODE: &str = "FTUI_GLYPH_MODE";
/// Environment variable to override emoji support (`1/0/true/false`).
const ENV_GLYPH_EMOJI: &str = "FTUI_GLYPH_EMOJI";
/// Legacy environment variable to disable emoji (`1/0/true/false`).
const ENV_NO_EMOJI: &str = "FTUI_NO_EMOJI";

/// Overall glyph rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphMode {
    /// Use Unicode glyphs (box drawing, symbols, arrows).
    Unicode,
    /// Use ASCII-only fallbacks.
    Ascii,
}

impl GlyphMode {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "unicode" | "uni" | "u" => Some(Self::Unicode),
            "ascii" | "ansi" | "a" => Some(Self::Ascii),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unicode => "unicode",
            Self::Ascii => "ascii",
        }
    }
}

/// Glyph capability policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphPolicy {
    /// Overall glyph mode (Unicode vs ASCII).
    pub mode: GlyphMode,
    /// Whether emoji glyphs should be used.
    pub emoji: bool,
    /// Whether ambiguous-width glyphs should be treated as double-width.
    pub cjk_width: bool,
    /// Whether Unicode line drawing should be used.
    pub unicode_line_drawing: bool,
    /// Whether Unicode arrows/symbols should be used.
    pub unicode_arrows: bool,
}

impl GlyphPolicy {
    /// Detect policy using environment variables and detected terminal caps.
    #[must_use]
    pub fn detect() -> Self {
        let caps = TerminalCapabilities::detect();
        Self::from_env_with(|key| std::env::var(key).ok(), &caps)
    }

    /// Detect policy using a custom environment lookup (for tests).
    #[must_use]
    pub fn from_env_with<F>(get_env: F, caps: &TerminalCapabilities) -> Self
    where
        F: Fn(&str) -> Option<String>,
    {
        let mode = detect_mode(&get_env, caps);
        let emoji = detect_emoji(&get_env, caps, mode);
        let cjk_width = text_width::cjk_width_from_env(get_env);

        let unicode_line_drawing = mode == GlyphMode::Unicode;
        let unicode_arrows = mode == GlyphMode::Unicode;

        Self {
            mode,
            emoji,
            cjk_width,
            unicode_line_drawing,
            unicode_arrows,
        }
    }

    /// Serialize policy to JSON (for diagnostics/evidence logs).
    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                r#"{{"glyph_mode":"{}","emoji":{},"cjk_width":{},"unicode_line_drawing":{},"unicode_arrows":{}}}"#
            ),
            self.mode.as_str(),
            self.emoji,
            self.cjk_width,
            self.unicode_line_drawing,
            self.unicode_arrows
        )
    }
}

fn detect_mode<F>(get_env: &F, caps: &TerminalCapabilities) -> GlyphMode
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(value) = get_env(ENV_GLYPH_MODE)
        && let Some(parsed) = GlyphMode::parse(&value)
    {
        return parsed;
    }

    match caps.profile() {
        TerminalProfile::Dumb | TerminalProfile::Vt100 | TerminalProfile::LinuxConsole => {
            GlyphMode::Ascii
        }
        _ => GlyphMode::Unicode,
    }
}

fn detect_emoji<F>(get_env: &F, caps: &TerminalCapabilities, mode: GlyphMode) -> bool
where
    F: Fn(&str) -> Option<String>,
{
    if mode == GlyphMode::Ascii {
        return false;
    }

    if let Some(value) = get_env(ENV_GLYPH_EMOJI)
        && let Some(parsed) = parse_bool(&value)
    {
        return parsed;
    }

    if let Some(value) = get_env(ENV_NO_EMOJI)
        && let Some(parsed) = parse_bool(&value)
    {
        return !parsed;
    }

    if matches!(
        caps.profile(),
        TerminalProfile::Dumb | TerminalProfile::Vt100 | TerminalProfile::LinuxConsole
    ) {
        return false;
    }

    let term_program = get_env("TERM_PROGRAM")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if is_known_emoji_terminal(&term_program) {
        return true;
    }

    let term = get_env("TERM").unwrap_or_default().to_ascii_lowercase();
    if is_known_emoji_terminal(&term) {
        return true;
    }

    // Default to true; users can explicitly disable.
    true
}

fn is_known_emoji_terminal(value: &str) -> bool {
    value.contains("wezterm")
        || value.contains("alacritty")
        || value.contains("iterm")
        || value.contains("ghostty")
        || value.contains("kitty")
        || value.contains("xterm")
        || value.contains("256color")
        || value.contains("vscode")
        || value.contains("rio")
        || value.contains("hyper")
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn map_env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    fn get_env<'a>(map: &'a HashMap<String, String>) -> impl Fn(&str) -> Option<String> + 'a {
        move |key| map.get(key).cloned()
    }

    #[test]
    fn glyph_mode_ascii_forces_ascii_policy() {
        let env = map_env(&[(ENV_GLYPH_MODE, "ascii"), ("TERM", "xterm-256color")]);
        let caps = TerminalCapabilities::modern();
        let policy = GlyphPolicy::from_env_with(get_env(&env), &caps);

        assert_eq!(policy.mode, GlyphMode::Ascii);
        assert!(!policy.unicode_line_drawing);
        assert!(!policy.unicode_arrows);
        assert!(!policy.emoji);
    }

    #[test]
    fn emoji_override_disable() {
        let env = map_env(&[(ENV_GLYPH_EMOJI, "0"), ("TERM", "wezterm")]);
        let caps = TerminalCapabilities::modern();
        let policy = GlyphPolicy::from_env_with(get_env(&env), &caps);

        assert!(!policy.emoji);
    }

    #[test]
    fn emoji_default_true_for_modern_term() {
        let env = map_env(&[("TERM", "xterm-256color")]);
        let caps = TerminalCapabilities::modern();
        let policy = GlyphPolicy::from_env_with(get_env(&env), &caps);

        assert!(policy.emoji);
    }

    #[test]
    fn cjk_width_respects_env_override() {
        let env = map_env(&[("FTUI_TEXT_CJK_WIDTH", "1")]);
        let caps = TerminalCapabilities::modern();
        let policy = GlyphPolicy::from_env_with(get_env(&env), &caps);

        assert!(policy.cjk_width);
    }
}

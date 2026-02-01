#![forbid(unsafe_code)]

//! Color profiles and downgrade logic.

use std::collections::HashMap;

use ftui_render::cell::PackedRgba;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColorProfile {
    Mono,
    Ansi16,
    Ansi256,
    TrueColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MonoColor {
    Black,
    White,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ansi16Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl Ansi16Color {
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Black => 0,
            Self::Red => 1,
            Self::Green => 2,
            Self::Yellow => 3,
            Self::Blue => 4,
            Self::Magenta => 5,
            Self::Cyan => 6,
            Self::White => 7,
            Self::BrightBlack => 8,
            Self::BrightRed => 9,
            Self::BrightGreen => 10,
            Self::BrightYellow => 11,
            Self::BrightBlue => 12,
            Self::BrightMagenta => 13,
            Self::BrightCyan => 14,
            Self::BrightWhite => 15,
        }
    }

    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Black => (0, 0, 0),
            Self::Red => (205, 0, 0),
            Self::Green => (0, 205, 0),
            Self::Yellow => (205, 205, 0),
            Self::Blue => (0, 0, 238),
            Self::Magenta => (205, 0, 205),
            Self::Cyan => (0, 205, 205),
            Self::White => (229, 229, 229),
            Self::BrightBlack => (127, 127, 127),
            Self::BrightRed => (255, 0, 0),
            Self::BrightGreen => (0, 255, 0),
            Self::BrightYellow => (255, 255, 0),
            Self::BrightBlue => (92, 92, 255),
            Self::BrightMagenta => (255, 0, 255),
            Self::BrightCyan => (0, 255, 255),
            Self::BrightWhite => (255, 255, 255),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminalColor {
    TrueColor(PackedRgba),
    Ansi256(u8),
    Ansi16(Ansi16Color),
    Mono(MonoColor),
}

/// Cached color downgrader for a specific terminal profile.
#[derive(Debug)]
pub struct ColorDowngrader {
    profile: ColorProfile,
    cache_256: HashMap<u32, u8>,
    cache_16: HashMap<u32, Ansi16Color>,
    cache_mono: HashMap<u32, MonoColor>,
}

impl ColorDowngrader {
    #[must_use]
    pub fn new(profile: ColorProfile) -> Self {
        Self {
            profile,
            cache_256: HashMap::new(),
            cache_16: HashMap::new(),
            cache_mono: HashMap::new(),
        }
    }

    #[must_use]
    pub const fn profile(&self) -> ColorProfile {
        self.profile
    }

    pub fn set_profile(&mut self, profile: ColorProfile) {
        if self.profile != profile {
            self.profile = profile;
            self.cache_256.clear();
            self.cache_16.clear();
            self.cache_mono.clear();
        }
    }

    #[must_use]
    pub fn downgrade(&mut self, color: PackedRgba) -> TerminalColor {
        match self.profile {
            ColorProfile::TrueColor => TerminalColor::TrueColor(color),
            ColorProfile::Ansi256 => TerminalColor::Ansi256(self.to_ansi256(color)),
            ColorProfile::Ansi16 => TerminalColor::Ansi16(self.to_ansi16(color)),
            ColorProfile::Mono => TerminalColor::Mono(self.to_mono(color)),
        }
    }

    #[must_use]
    pub fn to_ansi256(&mut self, color: PackedRgba) -> u8 {
        let key = color.0;
        if let Some(cached) = self.cache_256.get(&key) {
            return *cached;
        }
        let code = rgb_to_256(color.r(), color.g(), color.b());
        self.cache_256.insert(key, code);
        code
    }

    #[must_use]
    pub fn to_ansi16(&mut self, color: PackedRgba) -> Ansi16Color {
        let key = color.0;
        if let Some(cached) = self.cache_16.get(&key) {
            return *cached;
        }
        let mapped = rgb_to_ansi16(color.r(), color.g(), color.b());
        self.cache_16.insert(key, mapped);
        mapped
    }

    #[must_use]
    pub fn to_mono(&mut self, color: PackedRgba) -> MonoColor {
        let key = color.0;
        if let Some(cached) = self.cache_mono.get(&key) {
            return *cached;
        }
        let mapped = rgb_to_mono(color.r(), color.g(), color.b());
        self.cache_mono.insert(key, mapped);
        mapped
    }
}

impl Default for ColorDowngrader {
    fn default() -> Self {
        Self::new(ColorProfile::TrueColor)
    }
}

#[inline]
fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    if r == g && g == b {
        if r < 8 {
            return 16;
        }
        if r > 248 {
            return 231;
        }
        return 232 + ((r - 8) / 10).min(23);
    }

    let r6 = (u16::from(r) * 6 / 256) as u8;
    let g6 = (u16::from(g) * 6 / 256) as u8;
    let b6 = (u16::from(b) * 6 / 256) as u8;
    16 + 36 * r6 + 6 * g6 + b6
}

#[inline]
fn rgb_to_ansi16(r: u8, g: u8, b: u8) -> Ansi16Color {
    let mut best = Ansi16Color::Black;
    let mut best_dist = u32::MAX;

    for candidate in ANSI16_PALETTE {
        let (cr, cg, cb) = candidate.rgb();
        let dist = weighted_distance(r, g, b, cr, cg, cb);
        if dist < best_dist {
            best_dist = dist;
            best = candidate;
        }
    }

    best
}

#[inline]
fn rgb_to_mono(r: u8, g: u8, b: u8) -> MonoColor {
    let luma = weighted_luma(r, g, b);
    if luma >= 128 {
        MonoColor::White
    } else {
        MonoColor::Black
    }
}

#[inline]
fn weighted_distance(r: u8, g: u8, b: u8, cr: u8, cg: u8, cb: u8) -> u32 {
    let dr = i32::from(r) - i32::from(cr);
    let dg = i32::from(g) - i32::from(cg);
    let db = i32::from(b) - i32::from(cb);

    let dr2 = (dr * dr) as u32;
    let dg2 = (dg * dg) as u32;
    let db2 = (db * db) as u32;

    dr2 * 2126 + dg2 * 7152 + db2 * 722
}

#[inline]
fn weighted_luma(r: u8, g: u8, b: u8) -> u8 {
    let luma = u32::from(r) * 2126 + u32::from(g) * 7152 + u32::from(b) * 722;
    (luma / 10000) as u8
}

const ANSI16_PALETTE: [Ansi16Color; 16] = [
    Ansi16Color::Black,
    Ansi16Color::Red,
    Ansi16Color::Green,
    Ansi16Color::Yellow,
    Ansi16Color::Blue,
    Ansi16Color::Magenta,
    Ansi16Color::Cyan,
    Ansi16Color::White,
    Ansi16Color::BrightBlack,
    Ansi16Color::BrightRed,
    Ansi16Color::BrightGreen,
    Ansi16Color::BrightYellow,
    Ansi16Color::BrightBlue,
    Ansi16Color::BrightMagenta,
    Ansi16Color::BrightCyan,
    Ansi16Color::BrightWhite,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_passthrough() {
        let mut downgrader = ColorDowngrader::new(ColorProfile::TrueColor);
        let color = PackedRgba::rgb(10, 20, 30);
        assert_eq!(downgrader.downgrade(color), TerminalColor::TrueColor(color));
    }

    #[test]
    fn rgb_to_256_grayscale_edges() {
        assert_eq!(rgb_to_256(0, 0, 0), 16);
        assert_eq!(rgb_to_256(255, 255, 255), 231);
        assert_eq!(rgb_to_256(8, 8, 8), 232);
    }

    #[test]
    fn rgb_to_256_color_cube() {
        assert_eq!(rgb_to_256(255, 0, 0), 196);
        assert_eq!(rgb_to_256(0, 255, 0), 46);
        assert_eq!(rgb_to_256(0, 0, 255), 21);
    }

    #[test]
    fn rgb_to_ansi16_basic_colors() {
        assert_eq!(rgb_to_ansi16(0, 0, 0), Ansi16Color::Black);
        assert_eq!(rgb_to_ansi16(255, 255, 255), Ansi16Color::BrightWhite);
    }

    #[test]
    fn rgb_to_mono_threshold() {
        assert_eq!(rgb_to_mono(0, 0, 0), MonoColor::Black);
        assert_eq!(rgb_to_mono(255, 255, 255), MonoColor::White);
    }

    #[test]
    fn cache_is_used_for_ansi256() {
        let mut downgrader = ColorDowngrader::new(ColorProfile::Ansi256);
        let color = PackedRgba::rgb(1, 2, 3);
        let first = downgrader.downgrade(color);
        let second = downgrader.downgrade(color);
        assert_eq!(first, second);
        assert_eq!(downgrader.cache_256.len(), 1);
    }

    #[test]
    fn cache_is_used_for_ansi16() {
        let mut downgrader = ColorDowngrader::new(ColorProfile::Ansi16);
        let color = PackedRgba::rgb(20, 40, 60);
        let _ = downgrader.downgrade(color);
        let _ = downgrader.downgrade(color);
        assert_eq!(downgrader.cache_16.len(), 1);
    }

    #[test]
    fn cache_is_used_for_mono() {
        let mut downgrader = ColorDowngrader::new(ColorProfile::Mono);
        let color = PackedRgba::rgb(120, 120, 120);
        let _ = downgrader.downgrade(color);
        let _ = downgrader.downgrade(color);
        assert_eq!(downgrader.cache_mono.len(), 1);
    }
}

#![no_main]

use arbitrary::Arbitrary;
use ftui_pty::virtual_terminal::{QuirkSet, VirtualTerminal};
use libfuzzer_sys::fuzz_target;

const MAX_SEQUENCE_COUNT: usize = 128;
const MAX_SEQUENCE_BYTES: usize = 256;
const MAX_TOTAL_BYTES: usize = 8192;
const MAX_SCROLLBACK: usize = 64;
const CSI_FINALS: &[u8] = b"ABCDEFGHJKLMPSTXZ`bdfghlmnr";
const DEC_MODES: &[u16] = &[6, 7, 25, 1047, 1048, 1049];
const UTF8_SAMPLES: &[&[u8]] = &[
    "é".as_bytes(),
    "中".as_bytes(),
    "🙂".as_bytes(),
    "👩‍💻".as_bytes(),
];

#[derive(Arbitrary, Debug)]
struct VtFuzzCase {
    width_seed: u8,
    height_seed: u8,
    quirk_seed: u8,
    sequences: Vec<VtSequence>,
}

#[derive(Arbitrary, Debug)]
enum VtSequence {
    Raw(Vec<u8>),
    Printable(Vec<u8>),
    Utf8Sample(u8),
    Csi { params: Vec<u16>, final_seed: u8 },
    DecMode { mode_seed: u8, enable: bool },
    OscTitle(Vec<u8>),
    Charset { slot_seed: u8, designator_seed: u8 },
    FullReset,
}

#[derive(Debug, PartialEq, Eq)]
struct TerminalFingerprint {
    cursor: (u16, u16),
    cursor_visible: bool,
    alternate_screen: bool,
    title: String,
    scrollback_len: usize,
    screen_text: String,
    sampled_cells: Vec<Option<(char, bool, bool)>>,
}

impl VtFuzzCase {
    fn dimensions(&self) -> (u16, u16) {
        let width = u16::from(self.width_seed % 120).max(1);
        let height = u16::from(self.height_seed % 40).max(1);
        (width, height)
    }

    fn quirks(&self) -> QuirkSet {
        QuirkSet::empty()
            .with_tmux_nested_cursor(self.quirk_seed & 0b001 != 0)
            .with_screen_immediate_wrap(self.quirk_seed & 0b010 != 0)
            .with_windows_no_alt_screen(self.quirk_seed & 0b100 != 0)
    }
}

impl VtSequence {
    fn to_chunk(&self) -> Vec<u8> {
        let mut out = Vec::new();
        match self {
            Self::Raw(bytes) => {
                out.extend(bytes.iter().take(MAX_SEQUENCE_BYTES).copied());
            }
            Self::Printable(bytes) => {
                out.extend(
                    bytes
                        .iter()
                        .take(MAX_SEQUENCE_BYTES)
                        .map(|byte| b' ' + (byte % 95)),
                );
            }
            Self::Utf8Sample(seed) => {
                out.extend_from_slice(UTF8_SAMPLES[usize::from(*seed) % UTF8_SAMPLES.len()]);
            }
            Self::Csi { params, final_seed } => {
                out.extend_from_slice(b"\x1b[");
                for (idx, param) in params.iter().take(8).enumerate() {
                    if idx > 0 {
                        out.push(b';');
                    }
                    out.extend_from_slice((param % 1000).to_string().as_bytes());
                }
                out.push(CSI_FINALS[usize::from(*final_seed) % CSI_FINALS.len()]);
            }
            Self::DecMode { mode_seed, enable } => {
                let mode = DEC_MODES[usize::from(*mode_seed) % DEC_MODES.len()];
                out.extend_from_slice(b"\x1b[?");
                out.extend_from_slice(mode.to_string().as_bytes());
                out.push(if *enable { b'h' } else { b'l' });
            }
            Self::OscTitle(bytes) => {
                out.extend_from_slice(b"\x1b]2;");
                out.extend(bytes.iter().take(128).map(|byte| b' ' + (byte % 95)));
                out.push(0x07);
            }
            Self::Charset {
                slot_seed,
                designator_seed,
            } => {
                let slots = [b'(', b')', b'*', b'+'];
                let designators = [b'B', b'0', b'A', b'<'];
                out.push(0x1b);
                out.push(slots[usize::from(*slot_seed) % slots.len()]);
                out.push(designators[usize::from(*designator_seed) % designators.len()]);
            }
            Self::FullReset => out.extend_from_slice(b"\x1bc"),
        }
        out
    }
}

fn build_chunks(case: &VtFuzzCase) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    let mut total_bytes = 0usize;

    for sequence in case.sequences.iter().take(MAX_SEQUENCE_COUNT) {
        if total_bytes >= MAX_TOTAL_BYTES {
            break;
        }

        let mut chunk = sequence.to_chunk();
        let remaining = MAX_TOTAL_BYTES - total_bytes;
        if chunk.len() > remaining {
            chunk.truncate(remaining);
        }
        total_bytes += chunk.len();
        chunks.push(chunk);
    }

    chunks
}

fn new_terminal(width: u16, height: u16, quirks: QuirkSet) -> VirtualTerminal {
    let mut vt = VirtualTerminal::with_quirks(width, height, quirks);
    vt.set_max_scrollback(MAX_SCROLLBACK);
    vt
}

fn assert_public_invariants(vt: &VirtualTerminal, width: u16, height: u16) {
    assert_eq!(vt.width(), width, "terminal width changed");
    assert_eq!(vt.height(), height, "terminal height changed");

    let (cursor_x, cursor_y) = vt.cursor();
    assert!(
        cursor_x <= width,
        "cursor x out of bounds: {cursor_x} > {width}"
    );
    assert!(
        cursor_y < height,
        "cursor y out of bounds: {cursor_y} >= {height}"
    );
    assert!(
        vt.scrollback_len() <= MAX_SCROLLBACK,
        "scrollback exceeded configured cap"
    );

    assert!(vt.cell_at(width, 0).is_none());
    assert!(vt.cell_at(0, height).is_none());
    assert!(
        vt.cell_at(width.saturating_sub(1), height.saturating_sub(1))
            .is_some()
    );
}

fn fingerprint(vt: &VirtualTerminal) -> TerminalFingerprint {
    let width = vt.width();
    let height = vt.height();
    let samples = [
        (0, 0),
        (width / 2, height / 2),
        (width.saturating_sub(1), height.saturating_sub(1)),
    ];

    TerminalFingerprint {
        cursor: vt.cursor(),
        cursor_visible: vt.cursor_visible(),
        alternate_screen: vt.is_alternate_screen(),
        title: vt.title().to_string(),
        scrollback_len: vt.scrollback_len(),
        screen_text: vt.screen_text(),
        sampled_cells: samples
            .into_iter()
            .map(|(x, y)| {
                vt.cell_at(x, y)
                    .map(|cell| (cell.ch, cell.style.bold, cell.style.italic))
            })
            .collect(),
    }
}

fuzz_target!(|case: VtFuzzCase| {
    let (width, height) = case.dimensions();
    let quirks = case.quirks();
    let chunks = build_chunks(&case);
    let flattened: Vec<u8> = chunks.iter().flatten().copied().collect();

    let mut whole = new_terminal(width, height, quirks);
    whole.feed(&flattened);
    assert_public_invariants(&whole, width, height);

    let mut chunked = new_terminal(width, height, quirks);
    for chunk in &chunks {
        chunked.feed(chunk);
        assert_public_invariants(&chunked, width, height);
    }

    assert_eq!(
        fingerprint(&chunked),
        fingerprint(&whole),
        "chunked feed diverged from whole-stream feed"
    );
});

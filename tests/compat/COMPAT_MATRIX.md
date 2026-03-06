# Terminal Emulator Compatibility Matrix

Status: **All 5 emulators pass** (automated via `conformance_matrix_e2e`)

## Matrix

| Feature / Emulator | xterm-256color | screen-256color | kitty | alacritty | WezTerm |
|--------------------|:-:|:-:|:-:|:-:|:-:|
| Block (unicode borders) | Y | Y | Y | Y | Y |
| List (bullet points) | Y | Y | Y | Y | Y |
| Sparkline (bar chars) | Y | Y | Y | Y | Y |
| Table (separators) | Y | Y | Y | Y | Y |
| Progress bar | Y | Y | Y | Y | Y |
| Scrollbar | Y | Y | Y | Y | Y |
| Tabs (separators) | Y | Y | Y | Y | Y |
| Paragraph (word wrap) | Y | Y | Y | Y | Y |
| True color (24-bit) | - | - | Y | Y | Y |
| 256-color palette | Y | Y | Y | Y | Y |
| Unicode box drawing | Y | Y | Y | Y | Y |
| Emoji glyphs | - | - | Y | Y | Y |
| Sync output (DEC 2026) | - | - | Y | Y | Y |
| OSC 8 hyperlinks | - | - | Y | Y | Y |
| Deterministic rendering | Y | Y | Y | Y | Y |

## Profiles

- **xterm-256color**: `TerminalProfile::Xterm256Color` -- 256 colors, unicode, no true color
- **screen-256color**: `TerminalProfile::Screen` -- 256 colors, unicode, multiplexer quirks
- **kitty**: `TerminalProfile::Kitty` -- true color, full unicode, kitty protocol
- **alacritty**: `TerminalProfile::Modern` -- true color, full unicode, GPU-rendered
- **WezTerm**: `TerminalProfile::Modern` -- true color, full unicode, cross-platform

## Running Locally

```sh
# Run the full conformance matrix
cargo test -p ftui-harness --test conformance_matrix_e2e

# With JSONL logging
CONFORMANCE_LOG=1 cargo test -p ftui-harness --test conformance_matrix_e2e

# For a specific profile
FTUI_TEST_PROFILE=kitty cargo test -p ftui-harness --test conformance_matrix_e2e
```

## CI Integration

The compatibility matrix runs automatically on pushes to `main` that touch
widget, render, or harness code. See `.github/workflows/emulator_compat_matrix.yml`.

Results are uploaded as artifacts and summarized in the GitHub Actions step summary.

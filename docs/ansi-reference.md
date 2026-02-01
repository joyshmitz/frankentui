# ANSI Escape Sequence Reference

This document provides a reference for the ANSI escape sequences used by FrankenTUI. It serves as the authoritative source for implementing the Presenter, TerminalWriter, and terminal-model tests.

## Notation

- `ESC` = `\x1b` (0x1B)
- `CSI` = `ESC [` (Control Sequence Introducer)
- `OSC` = `ESC ]` (Operating System Command)
- `ST` = `ESC \` or `\x07` (String Terminator)
- `{n}` = numeric parameter
- `{r}`, `{c}` = row, column (1-indexed)

### Direction Labels

Each sequence is annotated with its direction:

- **Emit** — ftui writes this sequence to the terminal
- **Parse** — ftui reads and interprets this sequence from terminal input
- **Both** — used in both directions

## SGR (Select Graphic Rendition) — Emit

Format: `CSI {code}[;{code}...] m`

### Reset and Attributes

| Sequence | Code | Effect |
|----------|------|--------|
| `CSI 0 m` | 0 | Reset all attributes |
| `CSI 1 m` | 1 | Bold / increased intensity |
| `CSI 2 m` | 2 | Dim / decreased intensity |
| `CSI 3 m` | 3 | Italic |
| `CSI 4 m` | 4 | Underline |
| `CSI 5 m` | 5 | Slow blink |
| `CSI 7 m` | 7 | Reverse video (swap fg/bg) |
| `CSI 8 m` | 8 | Hidden / invisible |
| `CSI 9 m` | 9 | Strikethrough |

### Attribute Reset

| Sequence | Code | Effect |
|----------|------|--------|
| `CSI 21 m` | 21 | Double underline (or bold off) |
| `CSI 22 m` | 22 | Normal intensity (bold/dim off) |
| `CSI 23 m` | 23 | Italic off |
| `CSI 24 m` | 24 | Underline off |
| `CSI 25 m` | 25 | Blink off |
| `CSI 27 m` | 27 | Reverse off |
| `CSI 28 m` | 28 | Hidden off |
| `CSI 29 m` | 29 | Strikethrough off |

### Basic Colors (3/4-bit)

**Foreground (30-37, 90-97):**

| Code | Color | Bright Code | Bright Color |
|------|-------|-------------|--------------|
| 30 | Black | 90 | Bright Black (Gray) |
| 31 | Red | 91 | Bright Red |
| 32 | Green | 92 | Bright Green |
| 33 | Yellow | 93 | Bright Yellow |
| 34 | Blue | 94 | Bright Blue |
| 35 | Magenta | 95 | Bright Magenta |
| 36 | Cyan | 96 | Bright Cyan |
| 37 | White | 97 | Bright White |
| 39 | Default | - | - |

**Background (40-47, 100-107):**

| Code | Color | Bright Code | Bright Color |
|------|-------|-------------|--------------|
| 40 | Black | 100 | Bright Black |
| 41 | Red | 101 | Bright Red |
| 42 | Green | 102 | Bright Green |
| 43 | Yellow | 103 | Bright Yellow |
| 44 | Blue | 104 | Bright Blue |
| 45 | Magenta | 105 | Bright Magenta |
| 46 | Cyan | 106 | Bright Cyan |
| 47 | White | 107 | Bright White |
| 49 | Default | - | - |

### 256-Color Mode (8-bit)

| Sequence | Effect |
|----------|--------|
| `CSI 38;5;{n} m` | Set foreground to color `n` (0-255) |
| `CSI 48;5;{n} m` | Set background to color `n` (0-255) |

**256-color palette:**
- 0-7: Standard colors (same as 30-37)
- 8-15: High-intensity colors (same as 90-97)
- 16-231: 6x6x6 RGB cube (`16 + 36*r + 6*g + b`, r/g/b in 0-5)
- 232-255: Grayscale (24 shades, dark to light)

### TrueColor Mode (24-bit)

| Sequence | Effect |
|----------|--------|
| `CSI 38;2;{r};{g};{b} m` | Set foreground to RGB |
| `CSI 48;2;{r};{g};{b} m` | Set background to RGB |

## Cursor Movement (Emit)

### Absolute Positioning

| Sequence | Name | Effect |
|----------|------|--------|
| `CSI {r};{c} H` | CUP | Move cursor to row `r`, column `c` |
| `CSI {r};{c} f` | HVP | Same as CUP |
| `CSI {c} G` | CHA | Move cursor to column `c` |
| `CSI {r} d` | VPA | Move cursor to row `r` |

### Relative Movement

| Sequence | Name | Effect |
|----------|------|--------|
| `CSI {n} A` | CUU | Move cursor up `n` rows |
| `CSI {n} B` | CUD | Move cursor down `n` rows |
| `CSI {n} C` | CUF | Move cursor forward `n` columns |
| `CSI {n} D` | CUB | Move cursor back `n` columns |
| `CSI {n} E` | CNL | Move to beginning of line `n` lines down |
| `CSI {n} F` | CPL | Move to beginning of line `n` lines up |

### Cursor Save/Restore (Emit)

| Sequence | Name | Effect |
|----------|------|--------|
| `ESC 7` | DECSC | Save cursor position + attributes (DEC) |
| `ESC 8` | DECRC | Restore cursor position + attributes (DEC) |
| `CSI s` | SCOSC | Save cursor position (ANSI) |
| `CSI u` | SCORC | Restore cursor position (ANSI) |

**Note:** DEC save/restore (`ESC 7`/`ESC 8`) is preferred as it saves more state and has wider support. See [ADR-001](adr/ADR-001-inline-mode.md).

## Erase Operations (Emit)

### Erase in Display (ED)

| Sequence | Effect |
|----------|--------|
| `CSI 0 J` | Erase from cursor to end of screen |
| `CSI 1 J` | Erase from start of screen to cursor |
| `CSI 2 J` | Erase entire screen |
| `CSI 3 J` | Erase entire screen + scrollback |

### Erase in Line (EL)

| Sequence | Effect |
|----------|--------|
| `CSI 0 K` | Erase from cursor to end of line |
| `CSI 1 K` | Erase from start of line to cursor |
| `CSI 2 K` | Erase entire line |

## Screen Modes (Emit)

### Alternate Screen Buffer

| Sequence | Effect |
|----------|--------|
| `CSI ? 1049 h` | Enable alternate screen buffer (save main, switch) |
| `CSI ? 1049 l` | Disable alternate screen buffer (restore main) |

### Cursor Visibility (Emit)

| Sequence | Name | Effect |
|----------|------|--------|
| `CSI ? 25 h` | DECTCEM | Show cursor |
| `CSI ? 25 l` | DECTCEM | Hide cursor |

### Cursor Style (Emit) — DECSCUSR

| Sequence | Effect |
|----------|--------|
| `CSI 0 SP q` | Default cursor style |
| `CSI 1 SP q` | Blinking block |
| `CSI 2 SP q` | Steady block |
| `CSI 3 SP q` | Blinking underline |
| `CSI 4 SP q` | Steady underline |
| `CSI 5 SP q` | Blinking bar (I-beam) |
| `CSI 6 SP q` | Steady bar (I-beam) |

**Note:** The `SP` is a literal space (0x20) before `q`.

## Synchronized Output (Emit) — DEC 2026

Reduces flicker by batching output updates.

| Sequence | Effect |
|----------|--------|
| `CSI ? 2026 h` | Begin synchronized update |
| `CSI ? 2026 l` | End synchronized update |

**Usage:**
```
CSI ? 2026 h    # Begin sync
... render UI ...
CSI ? 2026 l    # End sync (terminal now updates display)
```

**Notes:**
- Can be nested (terminal tracks nesting level)
- Terminals without support ignore these sequences
- FrankenTUI's terminal model tracks sync nesting for verification

## OSC 8 Hyperlinks (Emit)

Format: `OSC 8 ; {params} ; {uri} ST`

### Start Hyperlink

```
ESC ] 8 ; ; https://example.com BEL
```

Or with parameters:
```
ESC ] 8 ; id=mylink ; https://example.com BEL
```

### End Hyperlink

```
ESC ] 8 ; ; BEL
```

**Example:**
```
ESC]8;;https://example.com\x07Click here\x1b]8;;\x07
```

**Notes:**
- The `id` parameter groups related link segments
- Empty URI ends the hyperlink
- `ST` can be `BEL` (`\x07`) or `ESC \` (`\x1b\x5c`)

## Mouse Tracking (Both)

### Enable Mouse Modes (Emit)

| Sequence | Mode | Effect |
|----------|------|--------|
| `CSI ? 1000 h` | X10 | Button press only |
| `CSI ? 1002 h` | Button | Button press/release + motion while pressed |
| `CSI ? 1003 h` | Any | All mouse events including motion |
| `CSI ? 1006 h` | SGR | Enable SGR extended mouse encoding |

### Disable Mouse Modes (Emit)

| Sequence | Effect |
|----------|--------|
| `CSI ? 1000 l` | Disable X10 mode |
| `CSI ? 1002 l` | Disable button mode |
| `CSI ? 1003 l` | Disable any-event mode |
| `CSI ? 1006 l` | Disable SGR encoding |

### SGR Mouse Event Format (Parse)

Press: `CSI < {button};{x};{y} M`
Release: `CSI < {button};{x};{y} m`

**Button encoding:**
- 0: Left
- 1: Middle
- 2: Right
- 64: Scroll up
- 65: Scroll down
- +4: Shift held
- +8: Meta/Alt held
- +16: Control held
- +32: Motion event

## Bracketed Paste (Both)

### Enable/Disable (Emit)

| Sequence | Effect |
|----------|--------|
| `CSI ? 2004 h` | Enable bracketed paste mode |
| `CSI ? 2004 l` | Disable bracketed paste mode |

### Paste Boundaries (Parse)

When enabled, pasted text is wrapped:
- Start: `CSI 200 ~`
- End: `CSI 201 ~`

**Example input from terminal:**
```
\x1b[200~pasted text here\x1b[201~
```

## Focus Events (Both)

### Enable/Disable (Emit)

| Sequence | Effect |
|----------|--------|
| `CSI ? 1004 h` | Enable focus event reporting |
| `CSI ? 1004 l` | Disable focus event reporting |

### Focus Event Reports (Parse)

| Sequence | Event |
|----------|-------|
| `CSI I` | Terminal gained focus |
| `CSI O` | Terminal lost focus |

## Scroll Regions and Scrolling (Emit)

### DECSTBM — Set Top and Bottom Margins

| Sequence | Effect |
|----------|--------|
| `CSI {top};{bottom} r` | Set scroll region to rows `top` through `bottom` |
| `CSI r` | Reset scroll region to full screen |

### Scroll Up/Down

| Sequence | Name | Effect |
|----------|------|--------|
| `CSI {n} S` | SU | Scroll up `n` lines (content moves up, new blank lines at bottom) |
| `CSI {n} T` | SD | Scroll down `n` lines (content moves down, new blank lines at top) |

**Notes:**
- Used for inline mode scroll-region optimization
- Must be reset on exit/panic
- See [ADR-001](adr/ADR-001-inline-mode.md) for strategy details

## Kitty Keyboard Protocol (Both)

### Enable/Disable (Emit)

| Sequence | Effect |
|----------|--------|
| `CSI > {flags} u` | Push keyboard mode with flags |
| `CSI < u` | Pop keyboard mode |
| `CSI ? u` | Query current keyboard mode |

**Flags** (bitfield, combined with `|`):
- `1` — Disambiguate escape codes
- `2` — Report event types (press/repeat/release)
- `4` — Report alternate keys
- `8` — Report all keys as escape codes
- `16` — Report associated text

ftui uses `CSI > 15 u` (flags 1|2|4|8) when the terminal supports the protocol.

### Key Event Format (Parse)

| Format | Meaning |
|--------|---------|
| `CSI {keycode} u` | Basic key press |
| `CSI {keycode};{modifiers} u` | Key with modifiers |
| `CSI {keycode};{modifiers}:{event_type} u` | Key with event type |

**Event types:** `1` = press, `2` = repeat, `3` = release.

**Modifier encoding:** 1 + (shift=1, alt=2, ctrl=4, super=8, hyper=16, meta=32, capslock=64, numlock=128).

## Device Status Reports (Both)

### Query Cursor Position

| Sequence | Direction | Effect |
|----------|-----------|--------|
| `CSI 6 n` | Emit | Request cursor position |
| `CSI {r};{c} R` | Parse | Cursor position response |

### Query Device Attributes

| Sequence | Direction | Effect |
|----------|-----------|--------|
| `CSI c` | Emit | Request primary device attributes (DA1) |
| `CSI > c` | Emit | Request secondary device attributes (DA2) |
| `CSI > 0 q` | Emit | Request terminal version (XTVERSION) |

## Terminal Identification

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `TERM` | Terminal type (e.g., `xterm-256color`) |
| `TERM_PROGRAM` | Terminal application (e.g., `iTerm.app`) |
| `COLORTERM` | Color support (`truecolor`, `24bit`) |
| `TMUX` | tmux session info (if in tmux) |
| `STY` | screen session info (if in screen) |

## Terminal Setup and Teardown Order

`TerminalSession` (in `ftui-core`) enables and disables terminal modes in a specific
order. The teardown sequence is the reverse of setup, ensuring correct restoration
even on panic (via `Drop`).

### Setup (on entry)

```
CSI ? 1049 h           # Alternate screen (if fullscreen mode)
CSI ? 25 l             # Hide cursor
CSI ? 1000;1002;1006 h # Mouse tracking (if enabled)
CSI ? 2004 h           # Bracketed paste (if enabled)
CSI ? 1004 h           # Focus events (if enabled)
CSI > 15 u             # Kitty keyboard protocol (if supported)
```

### Teardown (on drop, reverse order)

```
CSI < u                # Pop kitty keyboard
CSI ? 1004 l           # Disable focus events
CSI ? 2004 l           # Disable bracketed paste
CSI ? 1000;1002;1006 l # Disable mouse
CSI ? 25 h             # Show cursor (always restored)
CSI ? 1049 l           # Leave alternate screen (if was enabled)
```

**Invariant:** If setup wrote a mode-enable sequence, teardown writes the corresponding
disable. Show-cursor is always emitted on teardown regardless of setup state.

## Output Sanitization

The `sanitize` module (`ftui-render/src/sanitize.rs`) strips control sequences from
untrusted content (e.g., subprocess output, LLM text) before rendering.

### Stripped sequences

| Category | Bytes | Reason |
|----------|-------|--------|
| ESC sequences | `\x1b` + CSI/OSC/DCS/APC | Prevents escape injection |
| C0 controls (most) | 0x00-0x08, 0x0B-0x0C, 0x0E-0x1A, 0x1C-0x1F | Non-printable |
| C1 controls | U+0080..U+009F | 8-bit equivalents of escape sequences |
| DEL | 0x7F | Non-printable |

### Preserved controls

| Byte | Character | Reason |
|------|-----------|--------|
| 0x09 | TAB | Needed for indentation |
| 0x0A | LF | Line breaks |
| 0x0D | CR | Carriage return (CRLF line endings) |

See [ADR-006](adr/ADR-006-untrusted-output-policy.md) for the policy rationale.

## Implementation Notes

### FrankenTUI Presenter Strategy

Per [ADR-002](adr/ADR-002-presenter-emission.md), the presenter uses **reset+apply** for style changes:

```rust
// For each style transition:
write!(w, "\x1b[0m")?;           // Reset
write!(w, "\x1b[{};...m", ...)?; // Apply new style
```

### Terminal Model Verification

The [terminal model](../crates/ftui-render/src/terminal_model.rs) parses and validates these sequences for testing:

- Cursor position stays in bounds
- SGR state doesn't leak between cells
- Hyperlinks are properly closed
- Synchronized output is balanced

## Source Code Locations

| File | Purpose |
|------|---------|
| [`crates/ftui-render/src/ansi.rs`](../crates/ftui-render/src/ansi.rs) | Pure byte-generation functions for all emitted sequences |
| [`crates/ftui-render/src/presenter.rs`](../crates/ftui-render/src/presenter.rs) | State-tracked ANSI emitter (consumes `ansi.rs` functions) |
| [`crates/ftui-core/src/input_parser.rs`](../crates/ftui-core/src/input_parser.rs) | CSI/OSC/SS3 input parser with DoS limits |
| [`crates/ftui-core/src/terminal_session.rs`](../crates/ftui-core/src/terminal_session.rs) | RAII terminal mode setup/teardown |
| [`crates/ftui-core/src/terminal_capabilities.rs`](../crates/ftui-core/src/terminal_capabilities.rs) | Feature detection (sync output, kitty keyboard) |
| [`crates/ftui-render/src/sanitize.rs`](../crates/ftui-render/src/sanitize.rs) | Untrusted output stripping |

## References

- [ECMA-48](https://www.ecma-international.org/publications-and-standards/standards/ecma-48/) - Control Functions for Coded Character Sets
- [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html) - Comprehensive reference
- [ANSI Escape Codes (Wikipedia)](https://en.wikipedia.org/wiki/ANSI_escape_code)
- [Terminal WG Specs](https://gitlab.freedesktop.org/terminal-wg/specifications) - Modern terminal specifications
- [Kitty Keyboard Protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) - Progressive enhancement keyboard protocol

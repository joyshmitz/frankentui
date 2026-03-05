# Fuzzing

This directory contains cargo-fuzz targets for FrankenTUI.

## Targets

| Target | Crate | What it fuzzes |
|--------|-------|---------------|
| `fuzz_input_parser` | ftui-core | Raw byte input parser (no-panic) |
| `fuzz_input_parser_structured` | ftui-core | Structured input events via Arbitrary |
| `fuzz_input_parser_long_seq` | ftui-core | Long input sequences |
| `fuzz_text_cluster_map` | ftui-text | Grapheme cluster mapping |
| `fuzz_text_shaped_layout` | ftui-text | Shaped text layout pipeline |
| `fuzz_text_wrap` | ftui-text | Text wrapping (Word/Char/WordChar/Optimal) |
| `fuzz_text_width` | ftui-text | Display width calculations |
| `fuzz_text_hyphenation` | ftui-text | Hyphenation break points |
| `fuzz_layout_constraints` | ftui-layout | Constraint solver (Flex split) |
| `fuzz_widget_render` | ftui-widgets | Widget rendering (Block/Paragraph/Sparkline/ProgressBar) |

## Run

```bash
# Run a single target
cargo +nightly fuzz run fuzz_layout_constraints -- -max_len=256
cargo +nightly fuzz run fuzz_widget_render -- -max_len=512
cargo +nightly fuzz run fuzz_text_wrap -- -max_len=512

# Run all targets briefly (smoke test)
for target in $(cargo +nightly fuzz list 2>/dev/null); do
  echo "--- $target ---"
  cargo +nightly fuzz run "$target" -- -max_total_time=10 -max_len=512
done
```

## Notes

- `fuzz/target/` contains build artifacts (ignored by git).
- `fuzz/artifacts/` contains crash reproducers (ignored by git).
- `fuzz_vt_parser` and `fuzz_grid_mutations` were removed (frankenterm-core crate no longer exists).

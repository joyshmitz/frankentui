# Hotspot Table — ranked by cumulative

| Rank | Location | Metric | Value | Category | Evidence |
|------|----------|--------|-------|----------|----------|
| 1 | `unicode_segmentation::Graphemes::next` | cumulative | 2.44s | CPU | Raw perf report: top symbol 7.99% of cycles |
| 2 | `core::str::pattern::StrSearcher::new via str::replace` | cumulative | 2.20s | CPU | Raw perf report: top symbol 7.21% of cycles; stack includes unicodeit replacement |
| 3 | `ftui_render::buffer::Buffer::set` | cumulative | 1.58s | CPU | Raw perf report: top symbol 5.18% of cycles |
| 4 | `profile_sweep::main mixed app view/render loop` | cumulative | 1.39s | CPU | Raw perf report: top symbol 4.56% of cycles |
| 5 | `ftui_render::buffer::Buffer::mark_dirty_span` | cumulative | 1.14s | CPU | Raw perf report: top symbol 3.73% of cycles |
| 6 | `ftui_demo_showcase::screens::quake::QuakeE1M1State::render closure` | cumulative | 1.04s | CPU | Raw perf report: top symbol 3.40% of cycles |
| 7 | `ftui_render::buffer::Buffer::set_fast` | cumulative | 981.7ms | CPU | Raw perf report: top symbol 3.21% of cycles |
| 8 | `hit-grid clone / libc memmove` | cumulative | 899.1ms | CPU | Raw perf report: `__memmove_avx` stack includes `AppModel::view -> cache_hit_grid -> clone` |
| 9 | `ftui_widgets::paragraph::Paragraph::text_hash` | cumulative | 718.7ms | CPU | Raw perf report: top symbol 2.35% of cycles |
| 10 | `unicode_display_width::width` | cumulative | 694.2ms | CPU | Raw perf report: top symbol 2.27% of cycles |
| 11 | `ftui_render::presenter::Presenter::emit_style_changes` | cumulative | 660.6ms | CPU | Raw perf report: top symbol 2.16% of cycles |
| 12 | `ftui_render::diff::scan_row_changes_blockwise` | cumulative | 614.7ms | CPU | Raw perf report: top symbol 2.01% of cycles |

## Hypothesis Ledger

- **Primary workload is I/O-bound** → `rejects` — `strace-profile-sweep-off.txt`: 199 syscalls and 0.001797s syscall time for 7,200 frames
- **FrameArena improves current full demo pipeline** → `rejects` — `hyperfine-rollup.json`: arena off mean 2.711s vs on mean 3.307s across three paired repeats
- **Diff/presenter kernel dominates whole-frame p99** → `rejects` — `profile-sweep-off-cycles200.json`: frame p99 4018us while presenter p99 132us; `presenter-pipeline-bench.txt`: 200x60 pipeline under 88us

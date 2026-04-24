# Hotspot Table — ranked by count

| Rank | Location | Metric | Value | Category | Evidence |
|------|----------|--------|-------|----------|----------|
| 1 | `unicodeit::latex_to_unicode replacement in ftui-extras markdown` | count | 6189740 | alloc | tests/artifacts/perf/20260424T161752Z-profile-sweep/heaptrack_profile_sweep_off_direct_report.txt: 6,189,740 allocation calls via ftui-extras/src/markdown.rs |
| 2 | `unicode_segmentation::Graphemes::next` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 7.99% of cycles |
| 3 | `core::str::pattern::StrSearcher::new via str::replace` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 7.21% of cycles; stack includes unicodeit replacement |
| 4 | `ftui_render::buffer::Buffer::set` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 5.18% of cycles |
| 5 | `profile_sweep::main mixed app view/render loop` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 4.56% of cycles |
| 6 | `ftui_render::buffer::Buffer::mark_dirty_span` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 3.73% of cycles |
| 7 | `ftui_demo_showcase::screens::quake::QuakeE1M1State::render closure` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 3.40% of cycles |
| 8 | `ftui_render::buffer::Buffer::set_fast` | count | 30250 | CPU | tests/artifacts/perf/20260424T161752Z-profile-sweep/perf_profile_sweep_off_report.txt: top symbol 3.21% of cycles |

## Hypothesis Ledger

- **Primary workload is I/O-bound** → `rejects` — strace_profile_sweep_off.txt: 199 syscalls and 0.001797s syscall time for 7,200 frames
- **FrameArena improves current full demo pipeline** → `rejects` — hyperfine_rollup.json: arena off mean 2.711s vs on mean 3.307s across three paired repeats
- **Diff/presenter kernel dominates whole-frame p99** → `rejects` — profile_sweep_off_cycles200.json: frame p99 4018us while presenter p99 132us; presenter_pipeline_bench.txt: 200x60 pipeline under 88us

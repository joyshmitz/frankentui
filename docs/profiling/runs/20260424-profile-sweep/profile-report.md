# Profiling Report

Run ID: `20260424T161752Z-profile-sweep`

## Scope

This pass profiled the FrankenTUI demo render pipeline and isolated render-kernel benches before handing off to `extreme-software-optimization`.

No optimization changes were made.

## Method

- Added and used Cargo profile `release-perf`.
- Captured host/toolchain fingerprint in `fingerprint.json`.
- Captured an OS-tuning dry run; no sudo tuning was applied. The raw dry-run
  file is not retained in Git.
- Ran paired hyperfine comparisons for `arena-mode off` and `arena-mode on`.
- Captured single-run JSON/resource stats for `18,000` frames per arena mode.
- Captured CPU samples with `samply` and `perf`.
- Captured allocation attribution with direct `heaptrack`.
- Captured syscall summary with `strace -c`.
- Ran focused `ftui-render` diff and presenter Criterion benches through `rch`.

## Main Results

`arena-mode off` is the current baseline:

- Hyperfine paired mean: `2.711s` off vs `3.307s` on for the 80-cycle workload.
- Variance: off p95 drift `3.6%` (`STABLE`); on p95 drift `7.5%` (`NOISE`).
- Resource run: `7.92s` wall, `44,732KB` max RSS, `99%` CPU.

Whole-pipeline `arena-mode off` at 200 cycles:

- `18,000` frames in `7.906s`.
- p50 `252us`, p95 `1,236us`, p99 `4,018us`, max `20,457us`.
- `28,476,383` allocations and `7.62GB` allocated.
- p99 allocations/frame `41,719`.
- presenter p99 `132us`.

Kernel isolation:

- `presenter-pipeline-bench.txt`: full diff+present is about `76.7us` for `200x60@5%` and `87.1us` for `200x60@50%`.
- `diff-span-sparse-stats-bench.txt`: `compute_dirty` is materially faster than full compute for sparse `200x60` cases.
- `strace-profile-sweep-off.txt`: only `199` syscalls and `0.001797s` syscall time.

## Hotspots

The top CPU and allocation findings are rendered in:

- `hotspot-table.md`
- `hotspot-table-by-count.md`

The strongest root cause is repeated markdown math conversion:

- The raw direct heaptrack report, omitted from Git, reported `6,189,740`
  allocation calls through `unicodeit::latex_to_unicode`.
- The stack flows through `crates/ftui-extras/src/markdown.rs` into `crates/ftui-demo-showcase/src/screens/markdown_rich_text.rs`.
- The raw perf report, omitted from Git, also placed
  `core::str::pattern::StrSearcher::new` / `str::replace` at `7.21%` of cycles.

## Tool Notes

- Samply JSON was captured successfully, but the exported JSON was not
  symbolicated enough for the main human-readable report and is not tracked.
- The raw perf report produced a useful symbol table, but emitted `addr2line`
  warnings and recorded `97` lost samples due to host/tooling limitations.
- `perf_profile_sweep_off.data` is a large raw local artifact (`480M`) and is
  intentionally not tracked.
- The first heaptrack run wrapped `taskset` and is not used for attribution; the direct run is the authoritative allocation report.

## Handoff

Use `optimization-handoff.md` for the next `extreme-software-optimization` pass.

# Markdown Math Cache Optimization Report

Run ID: `20260424T180619Z-math-cache`

## Summary

This pass optimized the markdown math conversion hotspot identified in
`tests/artifacts/perf/20260424T161752Z-profile-sweep/optimization_handoff.md`.

The final implementation keeps LaTeX conversion memoization bounded and scoped:

- `MarkdownRenderer` owns a 128-entry FIFO cache keyed by exact LaTeX source.
- Inline math, display math, and math-flavored code blocks all use the same cached path.
- Demo markdown screens now keep persistent themed renderers, cloning them only to apply per-frame width and table animation options.
- The rich text screen uses separate persistent renderers for static markdown and streaming markdown so streaming fragments do not evict the static panel's hot math entries.

## Before vs Final

Baseline artifact: `../20260424T161752Z-profile-sweep/profile_sweep_off_cycles200.json`

Final artifact: `profile_sweep_after_split_cycles200.json`

| Metric | Before | Final | Change |
|---|---:|---:|---:|
| elapsed_ms | 7905.631 | 5404.941 | -31.63% |
| renders_per_sec | 2276.858 | 3330.286 | +46.27% |
| total allocations | 28476383 | 12620357 | -55.68% |
| p99 allocations/frame | 41719 | 7247 | -82.63% |
| total allocated bytes | 7622989884 | 7257076386 | -4.80% |
| p99 frame time | 4018 us | 1830 us | -54.45% |
| presenter p99 | 132 us | 94 us | -28.79% |

Hyperfine comparison, 80-cycle profile_sweep:

| Artifact | Mean | Min | Max |
|---|---:|---:|---:|
| `../20260424T161752Z-profile-sweep/hyperfine_off_1.json` | 2.693 s | 2.574 s | 2.871 s |
| `hyperfine_after_split_cycles80.json` | 2.266 s | 2.170 s | 2.380 s |

Mean wall time improved by 15.86% in the 5-run hyperfine comparison.

## Heaptrack Result

Baseline direct heaptrack reported:

- `heaptrack_profile_sweep_off_direct_stderr.txt`: 11751058 allocation calls.
- `hotspot_table_by_count.md`: 6189740 allocation calls through the `unicodeit::latex_to_unicode` replacement stack in `ftui-extras/src/markdown.rs`.

Final direct heaptrack reported:

- `heaptrack_after_split_direct_stderr.txt`: 5622122 allocation calls.
- `heaptrack_after_split_direct_report.txt`: the largest remaining `latex_to_unicode` replacement stack is 25304 calls.
- `heaptrack_after_split_direct_report.txt`: no residual `markdown_live_editor` stack appears in the LaTeX conversion search; remaining LaTeX calls are from the rich-text streaming path.

## Notes

- `profile_sweep_after_cycles200.json` records the first renderer-scoped cache attempt. It was rejected because the profile got slower: the markdown screen built fresh renderers every frame, so the cache lifetime was too short.
- `profile_sweep_after_persistent_cycles200.json` and `profile_sweep_after_all_cycles200.json` are intermediate checkpoints before the final rich-text cache split.
- `heaptrack_after_persistent.zst.zst` tracked `taskset`, not the profiling child process, and is intentionally ignored for conclusions. The useful heaptrack artifacts are the `*_direct*` files.

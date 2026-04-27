# FrankenTUI Profile Sweep Scenario

Run ID: `20260424T161752Z-profile-sweep`

Purpose: rank the highest-impact runtime/render hotspots before any optimization work.

Primary workload:

- Binary: `ftui-demo-showcase` `profile_sweep`
- Build profile: `release-perf`
- Command: `./target/release-perf/profile_sweep --cycles 200 --render-mode pipeline --arena-mode off --json`
- Screens: every demo screen from `ftui_demo_showcase::screens::screen_ids()`
- Sizes: `80x24` and `120x40`
- Metrics: frame time p50/p95/p99/max, renders per second, allocations per frame, allocated bytes per frame, changed cells, presenter time, emitted bytes

Comparison workload:

- Same command with `--arena-mode on`
- Purpose: identify whether frame arena allocation changes are still material relative to the current render pipeline.

Render-kernel workload:

- Criterion benches: `ftui-render` `diff_bench` and `presenter_bench`
- Purpose: isolate buffer diff and ANSI presenter costs from demo application code.

Host tuning:

- No sudo-level machine tuning was applied.
- The tuning dry-run was captured so variance can be interpreted against the actual machine state.

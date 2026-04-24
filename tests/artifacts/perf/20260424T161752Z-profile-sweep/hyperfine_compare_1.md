| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `taskset -c 2 /data/tmp/cargo-target/release-perf/profile_sweep --cycles 80 --render-mode pipeline --arena-mode off --json >/dev/null` | 2.693 ± 0.114 | 2.574 | 2.871 | 1.00 |
| `taskset -c 2 /data/tmp/cargo-target/release-perf/profile_sweep --cycles 80 --render-mode pipeline --arena-mode on --json >/dev/null` | 3.432 ± 0.555 | 2.668 | 4.138 | 1.27 ± 0.21 |

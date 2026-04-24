| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `taskset -c 2 /data/tmp/cargo-target/release-perf/profile_sweep --cycles 80 --render-mode pipeline --arena-mode off --json >/dev/null` | 2.704 ± 0.036 | 2.663 | 2.767 | 1.00 |
| `taskset -c 2 /data/tmp/cargo-target/release-perf/profile_sweep --cycles 80 --render-mode pipeline --arena-mode on --json >/dev/null` | 3.264 ± 0.382 | 2.753 | 3.850 | 1.21 ± 0.14 |

| Command | Mean [s] | Min [s] | Max [s] | Relative |
|:---|---:|---:|---:|---:|
| `taskset -c 2 /data/tmp/cargo-target/release-perf/profile_sweep --cycles 80 --render-mode pipeline --arena-mode off --json >/dev/null` | 2.736 ± 0.104 | 2.602 | 2.910 | 1.00 |
| `taskset -c 2 /data/tmp/cargo-target/release-perf/profile_sweep --cycles 80 --render-mode pipeline --arena-mode on --json >/dev/null` | 3.226 ± 0.369 | 2.671 | 3.670 | 1.18 ± 0.14 |

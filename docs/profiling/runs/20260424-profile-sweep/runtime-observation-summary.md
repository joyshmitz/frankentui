# Runtime Observation Summary

Run ID: `20260424T161752Z-profile-sweep`

This file preserves the useful numbers from raw GNU time, strace, and variance
text captures without retaining command-output logs.

## Resource Runs

| Arena mode | Wall time | User time | System time | CPU | Max RSS | Minor faults | Context switches |
|---|---:|---:|---:|---:|---:|---:|---:|
| `off` | 7.92 s | 7.88 s | 0.03 s | 99% | 44,732 KB | 8,335 | 72 involuntary |
| `on` | 8.67 s | 8.63 s | 0.02 s | 99% | 44,148 KB | 8,316 | 5 voluntary, 73 involuntary |

## Syscall Summary

The `arena-mode off` strace summary recorded `199` syscalls and `0.001797s`
total syscall time over the profiled run, so syscall overhead was not material
for the observed frame tail.

| Top syscall | Calls | Time |
|---|---:|---:|
| `munmap` | 10 | 0.000564 s |
| `execve` | 1 | 0.000432 s |
| `mmap` | 27 | 0.000206 s |
| `brk` | 85 | 0.000202 s |
| `openat` | 7 | 0.000106 s |

## Variance

| Arena mode | p95 samples | Median p95 | Max drift | Verdict |
|---|---:|---:|---:|---|
| `off` | 2871.42 ms, 2910.42 ms, 2766.97 ms | 2871.42 ms | 3.6% | STABLE |
| `on` | 4138.27 ms, 3670.30 ms, 3849.87 ms | 3849.87 ms | 7.5% | NOISE within envelope |

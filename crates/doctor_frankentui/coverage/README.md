# doctor_frankentui Coverage Gate

This directory contains the reproducible coverage policy for `doctor_frankentui`.

## Tracked Policy

- `thresholds.toml` — required minimum coverage percentages.
- `e2e_jsonl_schema.json` — telemetry schema used by doctor workflow validation.

Generated coverage summaries are not committed. They contain run-specific paths
and belong under `target/doctor_frankentui_coverage/`, the explicit output
directory passed to the script, or uploaded CI artifacts.

## Local Command

From repo root:

```bash
./scripts/doctor_frankentui_coverage.sh
```

Optional output directory override:

```bash
./scripts/doctor_frankentui_coverage.sh /tmp/doctor_frankentui_coverage_gate
```

The script writes:

- `coverage_summary.json` (machine-readable source-of-truth)
- `coverage_gate_report.json` (threshold evaluation details)
- `coverage_gate_report.txt` (human-readable report)

and exits non-zero if any configured threshold fails.

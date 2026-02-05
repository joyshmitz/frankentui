# E2E Coverage Matrix and Artifact Checklist

This document defines the artifact checklist required for every E2E
screen/flow. The goal is deterministic, audit-friendly runs where every
suite records its output paths in JSONL, and CI fails when required
artifacts are missing.

If you add a new E2E suite, add it here and wire its artifact logging
via `jsonl_assert "artifact_<type>" "pass" "path=<path>"`.

## Artifact Checklist (Required)

Every E2E case must record these artifacts in JSONL:

- `artifact_log_dir`: directory containing all logs for the run
- `artifact_jsonl`: the JSONL file path (`$E2E_JSONL_FILE`)
- `artifact_pty_output`: PTY output capture (when PTY is used)
- `artifact_hash_registry`: golden checksum registry file (when used)
- `artifact_snapshot`: snapshot output file (when snapshots are produced)
- `artifact_summary_json`: summary JSON for multi-suite runners (when produced)

Notes:
- `jsonl_assert` automatically emits `artifact` JSONL events and will fail in
  CI/strict mode if a required artifact is missing.
- PTY runs already emit `pty_capture` events with `output_file` and
  `canonical_file`. Still record `artifact_pty_output` for clarity and CI checks.

## Suite-Level Checklist

### Harness PTY Suites (`tests/e2e/scripts/test_*.sh`)

Required artifacts:
- `artifact_log_dir` = `$E2E_LOG_DIR`
- `artifact_jsonl` = `$E2E_JSONL_FILE`
- `artifact_pty_output` = PTY capture output file(s)
- `artifact_hash_registry` = any golden checksum file used by the test

### Demo/Script E2E Suites (`scripts/*` and `scripts/e2e/*`)

Required artifacts:
- `artifact_log_dir` = `$E2E_LOG_DIR` (or suite-specific log dir)
- `artifact_jsonl` = `$E2E_JSONL_FILE` (or suite-specific JSONL)
- `artifact_summary_json` when an aggregate summary is produced
- `artifact_snapshot` for snapshot-producing suites
- `artifact_hash_registry` when golden checksums are used

## Known Suites and Additional Artifacts

This section records extra artifacts per suite beyond the baseline list.

| Suite | Extra Artifacts | Notes |
| --- | --- | --- |
| `scripts/demo_showcase_e2e.sh` | `artifact_env_log`, `artifact_vfx_jsonl`, `artifact_layout_inspector_jsonl`, `artifact_summary_txt` | Demo showcase produces environment log + per-screen JSONL |
| `scripts/e2e_test.sh` | `artifact_summary_json` | PTY runner summary |
| `tests/e2e/scripts/test_golden_resize.sh` | `artifact_hash_registry` | Golden checksum file under `tests/golden_checksums/` |
| `tests/e2e/scripts/test_resize_storm.sh` | `artifact_pty_output` | Emits frame capture + checksum logs |

If a suite emits a checksum (or hash) for determinism, record the path to the
source file used to compute it via `artifact_hash_registry` or `artifact_snapshot`.

## Wiring Guidance

Example usage in a script:

```bash
jsonl_assert "artifact_log_dir" "pass" "log_dir=$E2E_LOG_DIR"
jsonl_assert "artifact_jsonl" "pass" "jsonl=$E2E_JSONL_FILE"
jsonl_assert "artifact_pty_output" "pass" "output=$OUTPUT_FILE"
jsonl_assert "artifact_hash_registry" "pass" "hash_registry=$CHECKSUM_FILE"
```

CI behavior:
- In CI (or with `E2E_JSONL_VALIDATE=1`), missing artifacts fail the run with
  a clear error from `jsonl_assert`.

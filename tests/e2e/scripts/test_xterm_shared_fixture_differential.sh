#!/bin/bash
set -euo pipefail

# E2E: xterm.js shared-fixture differential gate (bd-2vr05.10.2)
#
# Runs the shared VT fixture corpus through FrankenTerm's virtual terminal and
# classifies the result against the xterm.js-compatible fixture baseline. The
# Rust runner emits per-fixture JSONL plus a summary JSON; this script validates
# the artifact contract so failures are immediately triageable.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/../lib"

USER_E2E_LOG_DIR="${E2E_LOG_DIR:-}"
USER_E2E_RESULTS_DIR="${E2E_RESULTS_DIR:-}"
USER_E2E_JSONL_FILE="${E2E_JSONL_FILE:-}"
USER_REPORT_OUT="${REPORT_OUT:-}"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"

BASE_LOG_DIR="${USER_E2E_LOG_DIR:-/tmp/ftui_e2e_logs}"
E2E_LOG_DIR="${BASE_LOG_DIR}/xterm_shared_fixture_differential"
E2E_RESULTS_DIR="${USER_E2E_RESULTS_DIR:-$E2E_LOG_DIR/results}"
export E2E_LOG_DIR E2E_RESULTS_DIR

mkdir -p "$E2E_LOG_DIR" "$E2E_RESULTS_DIR"

e2e_fixture_init "xterm_shared_fixture_differential"

E2E_JSONL_FILE="${USER_E2E_JSONL_FILE:-$E2E_LOG_DIR/xterm_shared_fixture_differential_${E2E_RUN_ID}.jsonl}"
REPORT_OUT="${USER_REPORT_OUT:-$E2E_RESULTS_DIR/xterm_shared_fixture_differential_report_${E2E_RUN_ID}.json}"
SUMMARY_OUT="$E2E_RESULTS_DIR/xterm_shared_fixture_differential_summary_${E2E_RUN_ID}.json"
RUN_LOG="$E2E_LOG_DIR/xterm_shared_fixture_differential_${E2E_RUN_ID}.log"
export E2E_JSONL_FILE REPORT_OUT SUMMARY_OUT RUN_LOG

export FTUI_XTERM_DIFF_JSONL="$E2E_JSONL_FILE"
export FTUI_XTERM_DIFF_SUMMARY_JSON="$SUMMARY_OUT"
export FTUI_XTERM_DIFF_RUN_ID="$E2E_RUN_ID"

REMOTE_TARGET_DIR="${REMOTE_TARGET_DIR:-${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/frankentui-xterm-diff-e2e}}"
export REMOTE_TARGET_DIR

echo "=== xterm.js Shared-Fixture Differential E2E Test ==="
echo "[xterm-diff] run_id=$E2E_RUN_ID"
echo "[xterm-diff] jsonl=$E2E_JSONL_FILE"
echo "[xterm-diff] summary=$SUMMARY_OUT"
echo "[xterm-diff] report=$REPORT_OUT"

run_cargo_test() {
    local cargo_env=(env CARGO_TARGET_DIR="$REMOTE_TARGET_DIR")
    local cargo_cmd=(
        cargo test
        -p ftui-pty
        --test vt_support_matrix_runner
        xterm_shared_fixture_differential_matches_reference_fixtures
        --
        --nocapture
    )

    if command -v "${RCH_BIN:-rch}" >/dev/null 2>&1; then
        "${RCH_BIN:-rch}" exec -- "${cargo_env[@]}" "${cargo_cmd[@]}"
    else
        "${cargo_env[@]}" "${cargo_cmd[@]}"
    fi
}

if ! run_cargo_test | tee "$RUN_LOG"; then
    echo "[FAIL] xterm shared-fixture differential Rust runner failed"
    exit 1
fi

"${E2E_PYTHON:-python3}" - "$E2E_JSONL_FILE" "$SUMMARY_OUT" "$REPORT_OUT" "$RUN_LOG" <<'PY'
import json
import sys
from pathlib import Path
from typing import Any

jsonl_path = Path(sys.argv[1])
summary_path = Path(sys.argv[2])
report_path = Path(sys.argv[3])
run_log = Path(sys.argv[4])


def fail(message: str) -> None:
    raise SystemExit(f"[FAIL] {message}")


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        fail(f"missing JSONL artifact: {path}")
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as exc:
            fail(f"invalid JSONL on line {line_number}: {exc}")
        if not isinstance(value, dict):
            fail(f"JSONL line {line_number} is not an object")
        rows.append(value)
    return rows


events = read_jsonl(jsonl_path)
if not summary_path.exists():
    fail(f"missing summary artifact: {summary_path}")

summary = json.loads(summary_path.read_text(encoding="utf-8"))
if not isinstance(summary, dict):
    fail("summary artifact must be a JSON object")

suite = "xterm_shared_fixture_differential"
run_starts = [event for event in events if event.get("event") == "run_start" and event.get("suite") == suite]
run_summaries = [event for event in events if event.get("event") == "run_summary" and event.get("suite") == suite]
fixture_results = [
    event for event in events if event.get("event") == "fixture_result" and event.get("suite") == suite
]

if len(run_starts) != 1:
    fail(f"expected exactly one run_start, got {len(run_starts)}")
if len(run_summaries) != 1:
    fail(f"expected exactly one run_summary, got {len(run_summaries)}")
if len(fixture_results) < 300:
    fail(f"expected at least 300 shared fixtures, got {len(fixture_results)}")

reference_engine = summary.get("reference_engine")
if not isinstance(reference_engine, str) or not reference_engine.startswith("xterm.js"):
    fail(f"summary reference_engine must identify xterm.js baseline, got {reference_engine!r}")

statuses: dict[str, int] = {}
for result in fixture_results:
    status = result.get("status")
    if not isinstance(status, str):
        fail(f"fixture_result missing string status: {result}")
    statuses[status] = statuses.get(status, 0) + 1
    for field in ("correlation_id", "fixture", "fixture_path", "comparison_domain"):
        if not isinstance(result.get(field), str) or not result[field]:
            fail(f"fixture_result missing {field}: {result}")
    if not isinstance(result.get("duration_ms"), int):
        fail(f"fixture_result duration_ms must be integer: {result}")
    if not isinstance(result.get("mismatch_count"), int):
        fail(f"fixture_result mismatch_count must be integer: {result}")
    if status == "known_mismatch" and result["mismatch_count"] == 0:
        fail(f"known_mismatch fixture has no diagnostic mismatches: {result}")

failed = statuses.get("fail", 0)
passed = statuses.get("pass", 0)
known = statuses.get("known_mismatch", 0)
total = passed + known + failed

if failed != 0:
    fail(f"xterm differential has {failed} unexpected failures")
if total != len(fixture_results):
    fail(f"fixture status accounting mismatch: total={total}, events={len(fixture_results)}")
if summary.get("total") != total:
    fail(f"summary total {summary.get('total')} did not match fixture total {total}")
if summary.get("passed") != passed:
    fail(f"summary passed {summary.get('passed')} did not match fixture passed {passed}")
if summary.get("known_mismatch") != known:
    fail(f"summary known_mismatch {summary.get('known_mismatch')} did not match fixture known {known}")
if summary.get("failed") != failed:
    fail(f"summary failed {summary.get('failed')} did not match fixture failed {failed}")
if run_summaries[0].get("total") != summary.get("total"):
    fail("run_summary JSONL total does not match summary artifact")

report = {
    "suite": suite,
    "status": "passed",
    "jsonl": str(jsonl_path),
    "summary": str(summary_path),
    "run_log": str(run_log),
    "total": total,
    "passed": passed,
    "known_mismatch": known,
    "failed": failed,
    "reference_engine": reference_engine,
    "comparison_domain": summary.get("comparison_domain"),
}
report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
print(f"[PASS] xterm shared-fixture differential validated {total} fixtures ({known} known mismatches)")
print(f"[PASS] report={report_path}")
PY

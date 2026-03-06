#!/bin/bash
set -euo pipefail

# E2E: Selection resilience stress suite under churn (bd-2vr05.4.5)
#
# Validates selection markers survive heavy output churn, resize storms,
# scrollback flooding, and screen clears. Verifies OSC52 clipboard payloads
# remain intact through the stress sequence.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/../lib"
SCENARIOS_DIR="$SCRIPT_DIR/../scenarios/remote"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/remote.sh"

export E2E_DETERMINISTIC="${E2E_DETERMINISTIC:-1}"
export E2E_TIME_STEP_MS="${E2E_TIME_STEP_MS:-100}"
export E2E_SEED="${E2E_SEED:-0}"

REMOTE_PORT="${REMOTE_PORT:-9248}"
REMOTE_LOG_DIR="${REMOTE_LOG_DIR:-$E2E_LOG_DIR/remote_selection_stress}"
mkdir -p "$REMOTE_LOG_DIR"

trap remote_cleanup EXIT

echo "=== Remote Selection Stress E2E Test ==="
SCENARIO="$SCENARIOS_DIR/selection_stress.json"
JSONL_OUT="$REMOTE_LOG_DIR/selection_stress.jsonl"
TRANSCRIPT_OUT="$REMOTE_LOG_DIR/selection_stress.transcript"
REPORT_OUT="$REMOTE_LOG_DIR/selection_stress_report.json"

print_repro() {
    echo "Repro command:"
    echo "  E2E_DETERMINISTIC=$E2E_DETERMINISTIC E2E_SEED=$E2E_SEED REMOTE_PORT=$REMOTE_PORT bash $SCRIPT_DIR/test_remote_selection_stress.sh"
    echo "Artifacts:"
    echo "  Scenario:   $SCENARIO"
    echo "  JSONL:      $JSONL_OUT"
    echo "  Transcript: $TRANSCRIPT_OUT"
    echo "  Report:     $REPORT_OUT"
    if [[ -n "${REMOTE_TELEMETRY_FILE:-}" ]]; then
        echo "  Telemetry:  $REMOTE_TELEMETRY_FILE"
    fi
}

python_ws_client="${E2E_PYTHON:-python3}"
if ! "$python_ws_client" "$LIB_DIR/ws_client.py" --self-test >/dev/null; then
    echo "[FAIL] ws_client self-tests failed"
    print_repro
    exit 1
fi

if ! remote_start --port "$REMOTE_PORT" --cols 100 --rows 30 --cmd /bin/sh; then
    echo "[FAIL] Unable to start bridge for selection-stress scenario"
    print_repro
    exit 1
fi
if ! remote_wait_ready; then
    echo "[FAIL] Bridge did not become ready for selection-stress scenario"
    print_repro
    exit 1
fi
echo "[OK] Bridge ready on port $REMOTE_PORT (PID=$REMOTE_BRIDGE_PID)"

RESULT="$(remote_run_scenario "$SCENARIO" \
    --jsonl "$JSONL_OUT" \
    --transcript "$TRANSCRIPT_OUT" \
    --summary 2>&1)" || {
    echo "[FAIL] Selection-stress scenario execution failed"
    echo "$RESULT"
    print_repro
    exit 1
}

OUTCOME="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin)["outcome"])' 2>/dev/null || echo "unknown")"
FRAMES="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("frames", 0))' 2>/dev/null || echo "0")"
ASSERTIONS_TOTAL="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("assertions_total", 0))' 2>/dev/null || echo "0")"
ASSERTIONS_FAILED="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("assertions_failed", 0))' 2>/dev/null || echo "0")"
WS_IN="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("ws_in_bytes", 0))' 2>/dev/null || echo "0")"
WS_OUT="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("ws_out_bytes", 0))' 2>/dev/null || echo "0")"
CHECKSUM="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("checksum_chain", ""))' 2>/dev/null || echo "")"

if [[ "$OUTCOME" != "pass" ]]; then
    echo "[FAIL] Selection-stress scenario outcome: $OUTCOME"
    echo "$RESULT"
    print_repro
    exit 1
fi
if [[ "${FRAMES:-0}" -lt 1 ]]; then
    echo "[FAIL] Expected at least one frame from selection-stress scenario, got: ${FRAMES:-0}"
    print_repro
    exit 1
fi
if [[ "${ASSERTIONS_FAILED:-0}" -ne 0 ]]; then
    echo "[FAIL] Scenario assertions failed: ${ASSERTIONS_FAILED}/${ASSERTIONS_TOTAL}"
    echo "$RESULT"
    print_repro
    exit 1
fi

# Transcript marker validation
python3 - "$TRANSCRIPT_OUT" <<'PY'
import sys
from pathlib import Path

path = Path(sys.argv[1])
text = path.read_bytes().decode("utf-8", errors="replace")
required = [
    "SELECTION_STRESS_START",
    "SELECTION_ANCHOR_POST_CHURN",
    "SELECTION_SURVIVED_RESIZE_STORM",
    "SELECTION_POST_CLEAR",
    "SELECTION_STRESS_END",
]
missing = [marker for marker in required if marker not in text]
if missing:
    raise SystemExit(f"missing transcript markers: {missing}")
PY

# JSONL structural validation
python3 - "$JSONL_OUT" <<'PY'
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
events = [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
if not events:
    raise SystemExit("selection-stress JSONL is empty")

run_starts = [e for e in events if e.get("type") == "run_start"]
if len(run_starts) != 1:
    raise SystemExit(f"expected one run_start event, got {len(run_starts)}")
run_ends = [e for e in events if e.get("type") == "run_end"]
if len(run_ends) != 1:
    raise SystemExit(f"expected one run_end event, got {len(run_ends)}")
if run_ends[0].get("status") != "passed":
    raise SystemExit(f"run_end status is not passed: {run_ends[0].get('status')}")

resize_inputs = [e for e in events if e.get("type") == "input" and e.get("input_type") == "resize"]
if len(resize_inputs) < 4:
    raise SystemExit(f"expected at least 4 resize input events, got {len(resize_inputs)}")

frame_events = [e for e in events if e.get("type") == "frame"]
if len(frame_events) < 3:
    raise SystemExit(f"expected at least 3 frame events, got {len(frame_events)}")
for event in frame_events:
    frame_hash = event.get("frame_hash")
    if not isinstance(frame_hash, str) or not frame_hash.startswith("sha256:"):
        raise SystemExit(f"frame event missing sha256 frame_hash: {event}")
PY

# Generate report
python3 - "$RESULT" "$JSONL_OUT" "$TRANSCRIPT_OUT" "$REPORT_OUT" "$SCRIPT_DIR" "$REMOTE_PORT" "$E2E_SEED" "$E2E_DETERMINISTIC" <<'PY'
import json
import sys
from pathlib import Path

result = json.loads(sys.argv[1])
jsonl_path = sys.argv[2]
transcript_path = sys.argv[3]
report_path = Path(sys.argv[4])
script_dir = sys.argv[5]
remote_port = sys.argv[6]
seed = sys.argv[7]
deterministic = sys.argv[8]

report = {
    "suite": "remote_selection_stress",
    "status": "pass",
    "scenario": result.get("scenario"),
    "outcome": result.get("outcome"),
    "frames": result.get("frames"),
    "ws_in_bytes": result.get("ws_in_bytes"),
    "ws_out_bytes": result.get("ws_out_bytes"),
    "checksum_chain": result.get("checksum_chain"),
    "assertions_total": result.get("assertions_total"),
    "assertions_failed": result.get("assertions_failed"),
    "artifacts": {
        "jsonl": jsonl_path,
        "transcript": transcript_path,
    },
    "repro_command": (
        f"E2E_DETERMINISTIC={deterministic} E2E_SEED={seed} "
        f"REMOTE_PORT={remote_port} bash {script_dir}/test_remote_selection_stress.sh"
    ),
}
report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
PY

echo "[PASS] Remote selection stress"
echo "  Outcome:    $OUTCOME"
echo "  Frames:     $FRAMES"
echo "  WS in/out:  ${WS_IN}/${WS_OUT}"
echo "  Assertions: ${ASSERTIONS_TOTAL} total, ${ASSERTIONS_FAILED} failed"
echo "  Checksum:   $CHECKSUM"
echo "  Report:     $REPORT_OUT"

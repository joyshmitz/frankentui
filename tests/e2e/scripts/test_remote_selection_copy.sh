#!/bin/bash
set -euo pipefail

# E2E: Remote selection+copy interaction markers over WebSocket (bd-2vr05.4.4)

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
export E2E_SEED="${E2E_SEED:-0}"

REMOTE_PORT="${REMOTE_PORT:-9245}"
REMOTE_LOG_DIR="${REMOTE_LOG_DIR:-$E2E_LOG_DIR/remote_selection_copy}"
mkdir -p "$REMOTE_LOG_DIR"

trap remote_cleanup EXIT

echo "=== Remote Selection+Copy E2E Test ==="

remote_start --port "$REMOTE_PORT" --cols 100 --rows 30 --cmd /bin/sh
remote_wait_ready
echo "[OK] Bridge ready on port $REMOTE_PORT"

SCENARIO="$SCENARIOS_DIR/selection_copy.json"
JSONL_OUT="$REMOTE_LOG_DIR/selection_copy.jsonl"
TRANSCRIPT_OUT="$REMOTE_LOG_DIR/selection_copy.transcript"

print_repro() {
    echo "Repro command:"
    echo "  E2E_DETERMINISTIC=$E2E_DETERMINISTIC E2E_SEED=$E2E_SEED REMOTE_PORT=$REMOTE_PORT bash $SCRIPT_DIR/test_remote_selection_copy.sh"
    echo "Artifacts:"
    echo "  Scenario:   $SCENARIO"
    echo "  JSONL:      $JSONL_OUT"
    echo "  Transcript: $TRANSCRIPT_OUT"
    if [[ -n "${REMOTE_TELEMETRY_FILE:-}" ]]; then
        echo "  Bridge:     $REMOTE_TELEMETRY_FILE"
    fi
}

RESULT="$(remote_run_scenario "$SCENARIO" \
    --jsonl "$JSONL_OUT" \
    --transcript "$TRANSCRIPT_OUT" \
    --summary 2>&1)" || {
    echo "[FAIL] Scenario execution failed"
    echo "$RESULT"
    print_repro
    exit 1
}

OUTCOME="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("outcome", "unknown"))' 2>/dev/null || echo "unknown")"
FRAMES="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("frames", 0))' 2>/dev/null || echo "0")"
WS_IN="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("ws_in_bytes", 0))' 2>/dev/null || echo "0")"
WS_OUT="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("ws_out_bytes", 0))' 2>/dev/null || echo "0")"
ASSERTIONS_TOTAL="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("assertions_total", 0))' 2>/dev/null || echo "0")"
ASSERTIONS_FAILED="$(echo "$RESULT" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("assertions_failed", 0))' 2>/dev/null || echo "0")"

if [[ "$OUTCOME" != "pass" ]]; then
    echo "[FAIL] Remote selection+copy scenario outcome: $OUTCOME"
    echo "$RESULT"
    print_repro
    exit 1
fi

if [[ "${ASSERTIONS_FAILED:-0}" -ne 0 ]]; then
    echo "[FAIL] Scenario assertions failed: ${ASSERTIONS_FAILED}/${ASSERTIONS_TOTAL}"
    echo "$RESULT"
    print_repro
    exit 1
fi

if [[ "${FRAMES:-0}" -lt 1 ]]; then
    echo "[FAIL] Expected at least one frame, got: ${FRAMES:-0}"
    print_repro
    exit 1
fi

assert_transcript_contains() {
    local marker="$1"
    if ! strings "$TRANSCRIPT_OUT" | grep -Fq "$marker"; then
        echo "[FAIL] Transcript missing marker: $marker"
        print_repro
        exit 1
    fi
}

assert_transcript_contains "SELECTION_COPY_START"
assert_transcript_contains "SELECTION_COPY_END"
assert_transcript_contains "COPY_PAYLOAD_B64:YWxwaGEgYmV0YSBnYW1tYQ=="
assert_transcript_contains "SELECTION_COPY_DONE"

echo "[PASS] Remote selection+copy markers validated"
echo "  WS in: $WS_IN bytes | WS out: $WS_OUT bytes | Frames: $FRAMES | Assertions: ${ASSERTIONS_TOTAL}/${ASSERTIONS_TOTAL}"

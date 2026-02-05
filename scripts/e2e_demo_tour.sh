#!/usr/bin/env bash
# E2E test for Guided Tour Mode (bd-iuvb.1)
#
# Generates JSONL logs with:
# - run_id, step_id, screen_id, duration_ms, seed, size, mode, caps_profile
# - action, outcome, checksum (if present)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB_DIR="$PROJECT_ROOT/tests/e2e/lib"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"

LOG_DIR="${PROJECT_ROOT}/target/e2e-logs"
e2e_fixture_init "tour"
RUN_ID="${E2E_RUN_ID}"
TIMESTAMP="$(e2e_log_stamp)"
LOG_FILE="${LOG_DIR}/guided_tour_${RUN_ID}_${TIMESTAMP}.jsonl"
STDOUT_LOG="${LOG_DIR}/guided_tour_${TIMESTAMP}.log"

mkdir -p "$LOG_DIR"
export E2E_LOG_DIR="$LOG_DIR"
export E2E_RESULTS_DIR="${E2E_RESULTS_DIR:-$LOG_DIR/results}"
export E2E_RUN_CMD="${E2E_RUN_CMD:-$0 $*}"
export E2E_JSONL_FILE="$LOG_FILE"
mkdir -p "$E2E_RESULTS_DIR"
jsonl_init

# -----------------------------------------------------------------------
# Environment info
# -----------------------------------------------------------------------

echo '=== Guided Tour E2E (bd-iuvb.1) ==='
echo "Date: $(e2e_timestamp)"
echo "Log: $LOG_FILE"
echo

# -----------------------------------------------------------------------
# Build
# -----------------------------------------------------------------------

echo "Building ftui-demo-showcase (debug)..."
build_start_ms="$(e2e_now_ms)"
jsonl_step_start "build"
if cargo build -p ftui-demo-showcase > "$STDOUT_LOG" 2>&1; then
    build_duration_ms=$(( $(e2e_now_ms) - build_start_ms ))
    jsonl_step_end "build" "success" "$build_duration_ms"
else
    build_duration_ms=$(( $(e2e_now_ms) - build_start_ms ))
    jsonl_step_end "build" "failed" "$build_duration_ms"
    echo "FAIL: Build failed (see $STDOUT_LOG)"
    jsonl_run_end "failed" "$build_duration_ms" 1
    exit 1
fi

# -----------------------------------------------------------------------
# Run guided tour
# -----------------------------------------------------------------------

echo "Running guided tour..."
cols="${COLUMNS:-}"
rows="${LINES:-}"
jsonl_set_context "alt" "$cols" "$rows" "${E2E_SEED:-0}"

CMD=(
    cargo run -p ftui-demo-showcase --
    --tour
    --tour-speed=1.0
    --tour-start-step=1
    --exit-after-ms=7000
)

ENV_VARS=(
    "FTUI_TOUR_REPORT_PATH=$LOG_FILE"
    "FTUI_TOUR_RUN_ID=$RUN_ID"
    "FTUI_TOUR_SEED=${E2E_SEED:-0}"
    "FTUI_TOUR_CAPS_PROFILE=${TERM:-unknown}"
    "FTUI_DEMO_SCREEN_MODE=alt"
)

run_start_ms="$(e2e_now_ms)"
jsonl_step_start "guided_tour"
if command -v timeout >/dev/null 2>&1; then
    if env "${ENV_VARS[@]}" timeout 12s "${CMD[@]}" >> "$STDOUT_LOG" 2>&1; then
        run_status="success"
    else
        run_status="failed"
    fi
else
    if env "${ENV_VARS[@]}" "${CMD[@]}" >> "$STDOUT_LOG" 2>&1; then
        run_status="success"
    else
        run_status="failed"
    fi
fi
run_duration_ms=$(( $(e2e_now_ms) - run_start_ms ))
jsonl_step_end "guided_tour" "$run_status" "$run_duration_ms"
if [[ "$run_status" != "success" ]]; then
    echo "FAIL: Run failed (see $STDOUT_LOG)"
    jsonl_run_end "failed" "$run_duration_ms" 1
    exit 1
fi

# -----------------------------------------------------------------------
# Verify JSONL output
# -----------------------------------------------------------------------

if ! grep -q '"event":"tour"' "$LOG_FILE"; then
    echo "FAIL: No tour JSONL entries found"
    exit 1
fi

if ! grep -q '"action":"start"' "$LOG_FILE"; then
    echo "FAIL: Missing tour start log entry"
    exit 1
fi

jsonl_assert "tour_jsonl_entries" "pass" "guided tour JSONL entries present"
jsonl_run_end "success" "$run_duration_ms" 0

echo "PASS: Guided tour logs captured at $LOG_FILE"

echo
exit 0

#!/usr/bin/env bash
# E2E test for hover jitter stabilization (bd-9n09)
#
# Tests:
# 1. Jitter sequences do not cause target flicker
# 2. Intentional crossing triggers switch within expected latency
# 3. Hysteresis band prevents boundary oscillation
#
# Output: JSONL log with env, capabilities, timings, seed, checksums

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB_DIR="$PROJECT_ROOT/tests/e2e/lib"

# shellcheck source=/dev/null
if [[ -f "$LIB_DIR/common.sh" ]]; then
    source "$LIB_DIR/common.sh"
fi
if [[ -f "$LIB_DIR/logging.sh" ]]; then
    source "$LIB_DIR/logging.sh"
fi
if ! declare -f e2e_timestamp >/dev/null 2>&1; then
    e2e_timestamp() { date -Iseconds; }
fi
if ! declare -f e2e_log_stamp >/dev/null 2>&1; then
    e2e_log_stamp() { date +%Y%m%d_%H%M%S; }
fi

e2e_fixture_init "hover_stabilizer"
LOG_DIR="${PROJECT_ROOT}/target/e2e-logs"
LOG_FILE="${LOG_DIR}/hover_stabilizer_${E2E_RUN_ID}_$(e2e_log_stamp).jsonl"
E2E_LOG_DIR="$LOG_DIR"
E2E_RESULTS_DIR="${E2E_RESULTS_DIR:-$LOG_DIR/results}"
E2E_JSONL_FILE="${E2E_JSONL_FILE:-$LOG_FILE}"
E2E_RUN_CMD="${E2E_RUN_CMD:-$0 $*}"
E2E_RUN_START_MS="${E2E_RUN_START_MS:-$(e2e_run_start_ms)}"
export E2E_LOG_DIR E2E_RESULTS_DIR E2E_JSONL_FILE E2E_RUN_CMD E2E_RUN_START_MS

mkdir -p "$LOG_DIR"
mkdir -p "$E2E_RESULTS_DIR"
jsonl_init
jsonl_assert "artifact_log_dir" "pass" "log_dir=$LOG_DIR"
jsonl_set_context "host" "${COLUMNS:-}" "${LINES:-}" "${E2E_SEED:-0}"

# -----------------------------------------------------------------------
# Environment info
# -----------------------------------------------------------------------

echo '=== Hover Stabilizer E2E Tests (bd-9n09) ==='
echo "Date: $(e2e_timestamp)"
echo "Log: $LOG_FILE"
echo

# Log environment (jsonl_init already emitted env + run_start)

# -----------------------------------------------------------------------
# Build
# -----------------------------------------------------------------------

echo "Building ftui-core (release)..."
build_start_ms="$(e2e_now_ms)"
jsonl_step_start "build_ftui_core"
if cargo build -p ftui-core --release 2>&1 | tail -1; then
    jsonl_step_end "build_ftui_core" "success" "$(( $(e2e_now_ms) - build_start_ms ))"
else
    jsonl_step_end "build_ftui_core" "failed" "$(( $(e2e_now_ms) - build_start_ms ))"
    echo "FAIL: Build failed"
    jsonl_run_end "failed" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-$(e2e_now_ms)} ))" 1
    exit 1
fi

# -----------------------------------------------------------------------
# Run unit tests with output capture
# -----------------------------------------------------------------------

echo "Running hover_stabilizer unit tests..."

unit_start_ms="$(e2e_now_ms)"
jsonl_step_start "hover_stabilizer_unit"
TEST_OUTPUT=$(cargo test -p ftui-core hover_stabilizer -- --nocapture 2>&1)
TEST_EXIT=$?

if [ $TEST_EXIT -eq 0 ]; then
    # Count passed tests
    PASSED=$(echo "$TEST_OUTPUT" | grep -c 'ok$' || true)
    jsonl_step_end "hover_stabilizer_unit" "success" "$(( $(e2e_now_ms) - unit_start_ms ))"
    echo "Unit tests: PASS ($PASSED tests)"
else
    jsonl_step_end "hover_stabilizer_unit" "failed" "$(( $(e2e_now_ms) - unit_start_ms ))"
    echo "Unit tests: FAIL"
    echo "$TEST_OUTPUT"
    jsonl_run_end "failed" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-$(e2e_now_ms)} ))" 1
    exit 1
fi

# -----------------------------------------------------------------------
# Property test: jitter stability rate
# -----------------------------------------------------------------------

echo "Running property test: jitter stability..."

# Extract jitter stability test result
prop_start_ms="$(e2e_now_ms)"
jsonl_step_start "jitter_stability_rate"
JITTER_OUTPUT=$(cargo test -p ftui-core hover_stabilizer::tests::jitter_stability_rate -- --nocapture 2>&1)

if echo "$JITTER_OUTPUT" | grep -q 'test result: ok'; then
    jsonl_step_end "jitter_stability_rate" "success" "$(( $(e2e_now_ms) - prop_start_ms ))"
    echo "Jitter stability: PASS (>99% stable under oscillation)"
else
    jsonl_step_end "jitter_stability_rate" "failed" "$(( $(e2e_now_ms) - prop_start_ms ))"
    echo "Jitter stability: FAIL"
    jsonl_run_end "failed" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-$(e2e_now_ms)} ))" 1
    exit 1
fi

# -----------------------------------------------------------------------
# Property test: crossing detection latency
# -----------------------------------------------------------------------

echo "Running property test: crossing detection latency..."

lat_start_ms="$(e2e_now_ms)"
jsonl_step_start "crossing_detection_latency"
LATENCY_OUTPUT=$(cargo test -p ftui-core hover_stabilizer::tests::crossing_detection_latency -- --nocapture 2>&1)

if echo "$LATENCY_OUTPUT" | grep -q 'test result: ok'; then
    jsonl_step_end "crossing_detection_latency" "success" "$(( $(e2e_now_ms) - lat_start_ms ))"
    echo "Crossing latency: PASS (<=3 frames)"
else
    jsonl_step_end "crossing_detection_latency" "failed" "$(( $(e2e_now_ms) - lat_start_ms ))"
    echo "Crossing latency: FAIL"
    jsonl_run_end "failed" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-$(e2e_now_ms)} ))" 1
    exit 1
fi

# -----------------------------------------------------------------------
# Summary
# -----------------------------------------------------------------------

echo
echo '=== E2E Summary ==='
echo "All tests: PASS"
jsonl_run_end "success" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-$(e2e_now_ms)} ))" 0
echo
echo "Log written to: $LOG_FILE"

# Print log summary
echo
echo "=== Log Contents ==="
cat "$LOG_FILE" | jq -c '.' 2>/dev/null || cat "$LOG_FILE"

exit 0

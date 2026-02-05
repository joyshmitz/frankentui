#!/bin/bash
# Command Palette E2E PTY Test Script (bd-39y4.8)
#
# Exercises the command palette in a real PTY with verbose JSONL logging.
# Validates: compilation, unit tests, integration tests, no-panic.
#
# Usage:
#   ./scripts/command_palette_e2e.sh [--verbose] [--quick]
#
# Exit codes:
#   0  All tests passed
#   1  One or more tests failed

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
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
if ! declare -f e2e_now_ms >/dev/null 2>&1; then
    e2e_now_ms() { date +%s%3N; }
fi

VERBOSE=false
QUICK=false

for arg in "$@"; do
    case "$arg" in
        --verbose|-v) VERBOSE=true ;;
        --quick)      QUICK=true ;;
        --help|-h)
            echo "Usage: $0 [--verbose] [--quick]"
            echo "  --verbose  Show full output"
            echo "  --quick    Skip compilation, run tests only"
            exit 0
            ;;
    esac
done

e2e_fixture_init "command_palette"
TIMESTAMP="$(e2e_log_stamp)"
LOG_DIR="${LOG_DIR:-/tmp/ftui_palette_e2e_${E2E_RUN_ID}_${TIMESTAMP}}"
E2E_LOG_DIR="$LOG_DIR"
E2E_RESULTS_DIR="${E2E_RESULTS_DIR:-$LOG_DIR/results}"
E2E_JSONL_FILE="${E2E_JSONL_FILE:-$LOG_DIR/command_palette_e2e.jsonl}"
E2E_RUN_CMD="${E2E_RUN_CMD:-$0 $*}"
E2E_RUN_START_MS="${E2E_RUN_START_MS:-$(e2e_run_start_ms)}"
export E2E_LOG_DIR E2E_RESULTS_DIR E2E_JSONL_FILE E2E_RUN_CMD E2E_RUN_START_MS
mkdir -p "$E2E_LOG_DIR" "$E2E_RESULTS_DIR"
jsonl_init
jsonl_assert "artifact_log_dir" "pass" "log_dir=$E2E_LOG_DIR"
jsonl_set_context "host" "${COLUMNS:-}" "${LINES:-}" "${E2E_SEED:-0}"

PASSED=0
FAILED=0
SKIPPED=0

# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------

run_step() {
    local name="$1"
    shift
    local step_start
    step_start=$(e2e_now_ms)
    jsonl_step_start "$name"

    local exit_code=0
    local output_file="$LOG_DIR/${name}.log"

    if $VERBOSE; then
        "$@" 2>&1 | tee "$output_file" || exit_code=$?
    else
        "$@" > "$output_file" 2>&1 || exit_code=$?
    fi

    local step_end
    step_end=$(e2e_now_ms)
    local elapsed=$(( step_end - step_start ))

    if [ "$exit_code" -eq 0 ]; then
        PASSED=$((PASSED + 1))
        jsonl_step_end "$name" "success" "$elapsed"
        printf "  %-50s  PASS  (%s ms)\n" "$name" "$elapsed"
    else
        FAILED=$((FAILED + 1))
        jsonl_step_end "$name" "failed" "$elapsed"
        printf "  %-50s  FAIL  (exit %s, %s ms)\n" "$name" "$exit_code" "$elapsed"
        echo "    Log: $output_file"
    fi
}

skip_step() {
    local name="$1"
    SKIPPED=$((SKIPPED + 1))
    jsonl_step_start "$name"
    jsonl_step_end "$name" "skipped" 0
    printf "  %-50s  SKIP\n" "$name"
}

# ===========================================================================
# Environment dump
# ===========================================================================

echo "=========================================="
echo " Command Palette E2E Tests (bd-39y4.8)"
echo "=========================================="
echo ""

echo "  Log directory: $LOG_DIR"
echo ""

# ===========================================================================
# Step 1: Compilation
# ===========================================================================

if ! $QUICK; then
    run_step "cargo_check" \
        cargo check -p ftui-demo-showcase --tests --quiet

    run_step "cargo_clippy" \
        cargo clippy -p ftui-demo-showcase --tests -- -D warnings --quiet
else
    skip_step "cargo_check"
    skip_step "cargo_clippy"
fi

# ===========================================================================
# Step 2: Command Palette Unit Tests
# ===========================================================================

run_step "unit_tests_command_palette" \
    cargo test -p ftui-widgets -- command_palette --quiet

# ===========================================================================
# Step 3: Command Palette E2E Integration Tests
# ===========================================================================

run_step "e2e_integration_tests" \
    cargo test -p ftui-demo-showcase --test command_palette_e2e -- --nocapture 2>"$LOG_DIR/e2e_stderr.jsonl"

# ===========================================================================
# Step 4: Snapshot Tests (Command Palette)
# ===========================================================================

run_step "snapshot_tests_palette" \
    cargo test -p ftui-demo-showcase --test screen_snapshots -- command_palette --quiet

# ===========================================================================
# Step 5: PTY Smoke Test (if binary builds)
# ===========================================================================

has_pty_support() {
    command -v script >/dev/null 2>&1
}

if has_pty_support && ! $QUICK; then
    # Build the demo binary
    run_step "build_demo_binary" \
        cargo build -p ftui-demo-showcase --quiet

    DEMO_BIN="$PROJECT_ROOT/target/debug/ftui-demo-showcase"
    if [ -x "$DEMO_BIN" ]; then
        run_step "pty_smoke_test" bash -c "
            export FTUI_DEMO_EXIT_AFTER_MS=2000
            timeout 10 script -q /dev/null -c '$DEMO_BIN' </dev/null >/dev/null 2>&1 || test \$? -eq 124
        "
    else
        skip_step "pty_smoke_test"
    fi
else
    skip_step "build_demo_binary"
    skip_step "pty_smoke_test"
fi

# ===========================================================================
# Summary
# ===========================================================================

echo ""
echo "=========================================="
TOTAL=$((PASSED + FAILED + SKIPPED))
echo "  Total: $TOTAL  Passed: $PASSED  Failed: $FAILED  Skipped: $SKIPPED"
echo "=========================================="
echo ""

run_status="success"
if [ "$FAILED" -ne 0 ]; then
    run_status="failed"
fi
jsonl_run_end "$run_status" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-$(e2e_now_ms)} ))" "$FAILED"

if [ "$FAILED" -gt 0 ]; then
    echo "  JSONL log: $E2E_JSONL_FILE"
    echo "  E2E stderr: $LOG_DIR/e2e_stderr.jsonl"
    exit 1
fi

exit 0

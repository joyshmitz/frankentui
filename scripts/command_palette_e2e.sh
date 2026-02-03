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

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
LOG_DIR="${LOG_DIR:-/tmp/ftui_palette_e2e_${TIMESTAMP}}"
mkdir -p "$LOG_DIR"

PASSED=0
FAILED=0
SKIPPED=0

# ---------------------------------------------------------------------------
# JSONL logging
# ---------------------------------------------------------------------------

jsonl() {
    local step="$1"
    shift
    local fields="\"ts\":\"$(date -Iseconds)\",\"step\":\"$step\""
    while (( $# >= 2 )); do
        fields="${fields},\"$1\":\"$2\""
        shift 2
    done
    echo "{${fields}}" >> "$LOG_DIR/e2e.jsonl"
    if $VERBOSE; then
        echo "{${fields}}" >&2
    fi
}

# ---------------------------------------------------------------------------
# Test runner
# ---------------------------------------------------------------------------

run_step() {
    local name="$1"
    shift
    local step_start
    step_start=$(date +%s%3N)

    jsonl "step_start" "name" "$name"

    local exit_code=0
    local output_file="$LOG_DIR/${name}.log"

    if $VERBOSE; then
        "$@" 2>&1 | tee "$output_file" || exit_code=$?
    else
        "$@" > "$output_file" 2>&1 || exit_code=$?
    fi

    local step_end
    step_end=$(date +%s%3N)
    local elapsed=$(( step_end - step_start ))

    if [ "$exit_code" -eq 0 ]; then
        PASSED=$((PASSED + 1))
        jsonl "step_pass" "name" "$name" "elapsed_ms" "$elapsed"
        printf "  %-50s  PASS  (%s ms)\n" "$name" "$elapsed"
    else
        FAILED=$((FAILED + 1))
        jsonl "step_fail" "name" "$name" "elapsed_ms" "$elapsed" "exit_code" "$exit_code"
        printf "  %-50s  FAIL  (exit %s, %s ms)\n" "$name" "$exit_code" "$elapsed"
        echo "    Log: $output_file"
    fi
}

skip_step() {
    local name="$1"
    SKIPPED=$((SKIPPED + 1))
    jsonl "step_skip" "name" "$name"
    printf "  %-50s  SKIP\n" "$name"
}

# ===========================================================================
# Environment dump
# ===========================================================================

echo "=========================================="
echo " Command Palette E2E Tests (bd-39y4.8)"
echo "=========================================="
echo ""

jsonl "env" \
    "project_root" "$PROJECT_ROOT" \
    "log_dir" "$LOG_DIR" \
    "rust_version" "$(rustc --version 2>/dev/null || echo N/A)" \
    "cargo_version" "$(cargo --version 2>/dev/null || echo N/A)" \
    "term" "${TERM:-unknown}" \
    "colorterm" "${COLORTERM:-unknown}"

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

jsonl "summary" \
    "total" "$TOTAL" \
    "passed" "$PASSED" \
    "failed" "$FAILED" \
    "skipped" "$SKIPPED" \
    "log_dir" "$LOG_DIR"

if [ "$FAILED" -gt 0 ]; then
    echo "  JSONL log: $LOG_DIR/e2e.jsonl"
    echo "  E2E stderr: $LOG_DIR/e2e_stderr.jsonl"
    exit 1
fi

exit 0

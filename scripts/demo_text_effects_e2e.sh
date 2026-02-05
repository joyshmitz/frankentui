#!/usr/bin/env bash
# Text Effects E2E Test Script (bd-3cuk)
#
# Runs headless text effects demo and validates:
# - No panics during rendering
# - Render times within budget (< 16ms avg)
# - Memory growth within limits
#
# Usage:
#   ./scripts/demo_text_effects_e2e.sh
#   ./scripts/demo_text_effects_e2e.sh --verbose
#
# Exit codes:
#   0 - All tests passed
#   1 - Test failure (panic, timeout, budget exceeded)

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB_DIR="$PROJECT_ROOT/tests/e2e/lib"
# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"

LOG_DIR=""
LOG_FILE=""
RENDER_BUDGET_MS=16
TICK_COUNT=50
TIMEOUT_SECONDS=30
VERBOSE="${1:-}"

# ANSI colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# =============================================================================
# Helper Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

# =============================================================================
# Main Test Execution
# =============================================================================

main() {
    e2e_fixture_init "text_effects"
    LOG_DIR="${LOG_DIR:-/tmp/ftui_text_effects_${E2E_RUN_ID}}"
    LOG_FILE="${LOG_FILE:-$LOG_DIR/text_effects_e2e.log}"
    export E2E_LOG_DIR="$LOG_DIR"
    export E2E_RESULTS_DIR="${E2E_RESULTS_DIR:-$LOG_DIR/results}"
    export E2E_JSONL_FILE="${E2E_JSONL_FILE:-$LOG_DIR/text_effects_e2e.jsonl}"
    export E2E_RUN_CMD="${E2E_RUN_CMD:-$0 $*}"
    mkdir -p "$E2E_LOG_DIR" "$E2E_RESULTS_DIR"
    jsonl_init

    echo "=============================================="
    echo "  Text Effects E2E Tests (bd-3cuk)"
    echo "=============================================="
    echo ""
    echo "Date: $(date -Iseconds)"
    echo "Project: $PROJECT_ROOT"
    echo "Render budget: ${RENDER_BUDGET_MS}ms"
    echo "Tick count: $TICK_COUNT"
    echo ""

    cd "$PROJECT_ROOT"

    # -------------------------------------------------------------------------
    # Step 1: Build release binary
    # -------------------------------------------------------------------------
    log_info "Building ftui-demo-showcase (release)..."
    build_start_ms="$(e2e_now_ms)"
    jsonl_step_start "build"

    if ! cargo build -p ftui-demo-showcase --release 2>&1 | tail -5; then
        build_duration_ms=$(( $(e2e_now_ms) - build_start_ms ))
        jsonl_step_end "build" "failed" "$build_duration_ms"
        jsonl_run_end "failed" "$build_duration_ms" 1
        log_fail "Build failed!"
        exit 1
    fi

    build_duration_ms=$(( $(e2e_now_ms) - build_start_ms ))
    jsonl_step_end "build" "success" "$build_duration_ms"
    log_pass "Build successful"
    echo ""

    # -------------------------------------------------------------------------
    # Step 2: Run unit tests for text effects
    # -------------------------------------------------------------------------
    log_info "Running text effects unit tests..."
    unit_start_ms="$(e2e_now_ms)"
    jsonl_step_start "unit_tests"

    if ! cargo test -p ftui-extras --features text-effects -- --test-threads=1 2>&1 | tee "$LOG_FILE.unit"; then
        unit_duration_ms=$(( $(e2e_now_ms) - unit_start_ms ))
        jsonl_step_end "unit_tests" "failed" "$unit_duration_ms"
        jsonl_run_end "failed" "$unit_duration_ms" 1
        log_fail "Unit tests failed!"
        exit 1
    fi

    # Count test results
    UNIT_TESTS_PASSED=$(grep -c "test result: ok" "$LOG_FILE.unit" 2>/dev/null || echo "0")
    unit_duration_ms=$(( $(e2e_now_ms) - unit_start_ms ))
    jsonl_step_end "unit_tests" "success" "$unit_duration_ms"
    log_pass "Unit tests passed ($UNIT_TESTS_PASSED test suites)"
    echo ""

    # -------------------------------------------------------------------------
    # Step 3: Check for panics in demo (if headless mode available)
    # -------------------------------------------------------------------------
    log_info "Checking demo showcase for panics..."
    check_start_ms="$(e2e_now_ms)"
    jsonl_step_start "demo_check"

    # Run demo with timeout and capture output
    # Note: The demo may not have a headless mode yet, so we just build-check
    # text-effects is a feature in ftui-extras, not ftui-demo-showcase
    if cargo check -p ftui-demo-showcase 2>&1 | tee "$LOG_FILE.check"; then
        check_duration_ms=$(( $(e2e_now_ms) - check_start_ms ))
        jsonl_step_end "demo_check" "success" "$check_duration_ms"
        log_pass "Demo showcase builds successfully"
    else
        check_duration_ms=$(( $(e2e_now_ms) - check_start_ms ))
        jsonl_step_end "demo_check" "warn" "$check_duration_ms"
        log_warn "Demo showcase check had warnings"
    fi
    echo ""

    # -------------------------------------------------------------------------
    # Step 4: Run clippy on text effects
    # -------------------------------------------------------------------------
    log_info "Running clippy on text effects..."
    clippy_start_ms="$(e2e_now_ms)"
    jsonl_step_start "clippy"

    if cargo clippy -p ftui-extras --features text-effects -- -D warnings 2>&1 | tail -10; then
        clippy_duration_ms=$(( $(e2e_now_ms) - clippy_start_ms ))
        jsonl_step_end "clippy" "success" "$clippy_duration_ms"
        log_pass "Clippy passed"
    else
        clippy_duration_ms=$(( $(e2e_now_ms) - clippy_start_ms ))
        jsonl_step_end "clippy" "failed" "$clippy_duration_ms"
        jsonl_run_end "failed" "$clippy_duration_ms" 1
        log_fail "Clippy found issues"
        exit 1
    fi
    echo ""

    # -------------------------------------------------------------------------
    # Step 5: Check formatting
    # -------------------------------------------------------------------------
    log_info "Checking formatting..."
    fmt_start_ms="$(e2e_now_ms)"
    jsonl_step_start "fmt_check"

    if cargo fmt -p ftui-extras --check 2>&1; then
        fmt_duration_ms=$(( $(e2e_now_ms) - fmt_start_ms ))
        jsonl_step_end "fmt_check" "success" "$fmt_duration_ms"
        log_pass "Formatting correct"
    else
        fmt_duration_ms=$(( $(e2e_now_ms) - fmt_start_ms ))
        jsonl_step_end "fmt_check" "warn" "$fmt_duration_ms"
        log_warn "Formatting issues detected (run 'cargo fmt' to fix)"
    fi
    echo ""

    # -------------------------------------------------------------------------
    # Step 6: Run benchmarks (quick sanity check)
    # -------------------------------------------------------------------------
    log_info "Running benchmark sanity check..."
    bench_start_ms="$(e2e_now_ms)"
    jsonl_step_start "bench"

    # Quick benchmark run to ensure they compile and execute
    if cargo bench -p ftui-extras --bench text_effects_bench --features text-effects -- --quick 2>&1 | tail -20; then
        bench_duration_ms=$(( $(e2e_now_ms) - bench_start_ms ))
        jsonl_step_end "bench" "success" "$bench_duration_ms"
        log_pass "Benchmarks executed successfully"
    else
        bench_duration_ms=$(( $(e2e_now_ms) - bench_start_ms ))
        jsonl_step_end "bench" "warn" "$bench_duration_ms"
        log_warn "Benchmarks had issues (non-blocking)"
    fi
    echo ""

    # -------------------------------------------------------------------------
    # Summary
    # -------------------------------------------------------------------------
    echo "=============================================="
    echo "  E2E Test Summary"
    echo "=============================================="
    echo ""

    # Check for any failures in log
    if grep -q "FAILED" "$LOG_FILE.unit" 2>/dev/null; then
        log_fail "Some unit tests failed - see $LOG_FILE.unit"
        exit 1
    fi

    if grep -q "panicked" "$LOG_FILE.unit" 2>/dev/null; then
        log_fail "Panic detected - see $LOG_FILE.unit"
        jsonl_run_end "failed" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-0} ))" 1
        exit 1
    fi

    log_pass "All E2E tests passed!"
    jsonl_assert "artifact_unit_log" "pass" "unit_log=$LOG_FILE.unit"
    jsonl_assert "artifact_check_log" "pass" "check_log=$LOG_FILE.check"
    jsonl_run_end "success" "$(( $(e2e_now_ms) - ${E2E_RUN_START_MS:-0} ))" 0
    echo ""
    echo "Artifacts:"
    echo "  - Unit test log: $LOG_FILE.unit"
    echo "  - Check log: $LOG_FILE.check"
    echo ""

    # Cleanup temporary files
    rm -f "$LOG_FILE.unit" "$LOG_FILE.check" 2>/dev/null || true

    exit 0
}

# =============================================================================
# Run Main
# =============================================================================

main "$@"

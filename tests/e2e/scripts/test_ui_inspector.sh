#!/bin/bash
set -euo pipefail

# E2E tests for UI Inspector overlay (bd-17h9.3)
# - Smoke render at multiple sizes
# - Validate inspector panel text and labels are present

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/../lib"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/pty.sh"

JSONL_FILE="$E2E_RESULTS_DIR/ui_inspector.jsonl"
RUN_ID="ui_inspector_$(date +%Y%m%d_%H%M%S)_$$"
SEED="${FTUI_HARNESS_SEED:-0}"

jsonl_log() {
    local line="$1"
    mkdir -p "$E2E_RESULTS_DIR"
    printf '%s\n' "$line" >> "$JSONL_FILE"
}

sha256_file() {
    local file="$1"
    if command -v sha256sum >/dev/null 2>&1 && [[ -f "$file" ]]; then
        sha256sum "$file" | awk '{print $1}'
        return 0
    fi
    echo ""
    return 0
}

if [[ ! -x "${E2E_HARNESS_BIN:-}" ]]; then
    LOG_FILE="$E2E_LOG_DIR/ui_inspector_missing.log"
    for t in ui_inspector_120x40 ui_inspector_80x24; do
        log_test_skip "$t" "ftui-harness binary missing"
        record_result "$t" "skipped" 0 "$LOG_FILE" "binary missing"
        jsonl_log "{\"run_id\":\"$RUN_ID\",\"case\":\"$t\",\"status\":\"skipped\",\"reason\":\"binary missing\",\"seed\":\"$SEED\"}"
    done
    exit 0
fi

run_case() {
    local name="$1"
    local cols="$2"
    local rows="$3"
    shift 3
    local start_ms
    start_ms="$(date +%s%3N)"

    LOG_FILE="$E2E_LOG_DIR/${name}.log"
    local output_file="$E2E_LOG_DIR/${name}.pty"

    log_test_start "$name"

    if "$@"; then
        local end_ms
        end_ms="$(date +%s%3N)"
        local duration_ms=$((end_ms - start_ms))
        local size
        size=$(wc -c < "$output_file" | tr -d ' ')
        local output_sha
        output_sha="$(sha256_file "$output_file")"
        log_test_pass "$name"
        record_result "$name" "passed" "$duration_ms" "$LOG_FILE"
        jsonl_log "{\"run_id\":\"$RUN_ID\",\"case\":\"$name\",\"status\":\"passed\",\"duration_ms\":$duration_ms,\"output_bytes\":$size,\"output_sha256\":\"$output_sha\",\"seed\":\"$SEED\",\"view\":\"widget-inspector\",\"cols\":$cols,\"rows\":$rows,\"term\":\"${TERM:-}\",\"colorterm\":\"${COLORTERM:-}\",\"no_color\":\"${NO_COLOR:-}\"}"
        return 0
    fi

    local end_ms
    end_ms="$(date +%s%3N)"
    local duration_ms=$((end_ms - start_ms))
    local output_sha
    output_sha="$(sha256_file "$output_file")"
    log_test_fail "$name" "assertion failed"
    record_result "$name" "failed" "$duration_ms" "$LOG_FILE" "assertion failed"
    jsonl_log "{\"run_id\":\"$RUN_ID\",\"case\":\"$name\",\"status\":\"failed\",\"duration_ms\":$duration_ms,\"output_sha256\":\"$output_sha\",\"seed\":\"$SEED\",\"view\":\"widget-inspector\",\"cols\":$cols,\"rows\":$rows,\"term\":\"${TERM:-}\",\"colorterm\":\"${COLORTERM:-}\",\"no_color\":\"${NO_COLOR:-}\"}"
    return 1
}

ui_inspector_smoke() {
    local name="$1"
    local cols="$2"
    local rows="$3"
    local output_file="$E2E_LOG_DIR/${name}.pty"

    PTY_COLS="$cols" \
    PTY_ROWS="$rows" \
    FTUI_HARNESS_VIEW="widget-inspector" \
    FTUI_HARNESS_SUPPRESS_WELCOME=1 \
    FTUI_HARNESS_EXIT_AFTER_MS=1200 \
    PTY_TIMEOUT=4 \
        pty_run "$output_file" "$E2E_HARNESS_BIN"

    local size
    size=$(wc -c < "$output_file" | tr -d ' ')
    [[ "$size" -gt 300 ]] || return 1
    grep -a -q "Inspector" "$output_file" || return 1
    grep -a -q "Region:" "$output_file" || return 1
    grep -a -q "LogPanel" "$output_file" || return 1
}

FAILURES=0
run_case "ui_inspector_120x40" 120 40 ui_inspector_smoke "ui_inspector_120x40" 120 40 || FAILURES=$((FAILURES + 1))
run_case "ui_inspector_80x24" 80 24 ui_inspector_smoke "ui_inspector_80x24" 80 24 || FAILURES=$((FAILURES + 1))

if [[ "$FAILURES" -gt 0 ]]; then
    exit 1
fi

exit 0

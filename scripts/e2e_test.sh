#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
LIB_DIR="$PROJECT_ROOT/tests/e2e/lib"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/pty.sh"

VERBOSE=false
QUICK=false
RUN_LARGE=true
RUN_BUDGETED=true
ARGS=()

for arg in "$@"; do
    case "$arg" in
        --verbose|-v)
            VERBOSE=true
            LOG_LEVEL="DEBUG"
            ;;
        --quick|-q)
            QUICK=true
            ;;
        --no-large)
            RUN_LARGE=false
            ;;
        --no-budget)
            RUN_BUDGETED=false
            ;;
        --help|-h)
            echo "Usage: $0 [--verbose] [--quick] [--no-large] [--no-budget]"
            echo ""
            echo "Options:"
            echo "  --verbose, -v   Enable debug logging"
            echo "  --quick, -q     Run only core tests (inline + cleanup)"
            echo "  --no-large      Skip large-screen scenarios"
            echo "  --no-budget     Skip budgeted refresh scenario"
            echo "  --help, -h      Show this help"
            exit 0
            ;;
    esac
    ARGS+=("$arg")
done

if $QUICK; then
    RUN_LARGE=false
    RUN_BUDGETED=false
fi

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
E2E_LOG_DIR="${E2E_LOG_DIR:-/tmp/ftui_e2e_${TIMESTAMP}}"
if [[ -e "$E2E_LOG_DIR" ]]; then
    base="$E2E_LOG_DIR"
    suffix=1
    while [[ -e "${base}_$suffix" ]]; do
        suffix=$((suffix + 1))
    done
    E2E_LOG_DIR="${base}_$suffix"
fi
E2E_RESULTS_DIR="$E2E_LOG_DIR/results"
LOG_FILE="$E2E_LOG_DIR/e2e.log"
export E2E_LOG_DIR E2E_RESULTS_DIR LOG_FILE LOG_LEVEL
export E2E_RUN_START_MS="$(date +%s%3N)"

mkdir -p "$E2E_LOG_DIR" "$E2E_RESULTS_DIR"

log_info "FrankenTUI E2E launcher"
log_info "Project root: $PROJECT_ROOT"
log_info "Log directory: $E2E_LOG_DIR"
log_info "Mode: $([ "$QUICK" = true ] && echo quick || echo normal)"

set +e
"$PROJECT_ROOT/tests/e2e/scripts/run_all.sh" "${ARGS[@]}"
RUN_ALL_STATUS=$?
set -e

escape_json() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g; s/\r/\\r/g; s/\n/\\n/g'
}

record_terminal_caps() {
    local output_file="$1"
    {
        echo "Terminal Capabilities"
        echo "====================="
        echo "TERM=${TERM:-}"
        echo "COLORTERM=${COLORTERM:-}"
        echo "NO_COLOR=${NO_COLOR:-}"
        echo "TMUX=${TMUX:-}"
        echo "ZELLIJ=${ZELLIJ:-}"
        echo "KITTY_WINDOW_ID=${KITTY_WINDOW_ID:-}"
        echo "TERM_PROGRAM=${TERM_PROGRAM:-}"
        echo ""
        if command -v infocmp >/dev/null 2>&1; then
            echo "infocmp -1:"
            infocmp -1 2>/dev/null || true
        else
            echo "infocmp not available"
        fi
        echo ""
        echo "tput colors: $(tput colors 2>/dev/null || echo N/A)"
        echo "stty -a: $(stty -a 2>/dev/null || echo N/A)"
    } > "$output_file"
}

write_large_env() {
    local jsonl="$1"
    local seed="$2"
    local run_id="$3"
    cat >> "$jsonl" <<EOF
{"event":"large_screen_env","run_id":"$run_id","timestamp":"$(date -Iseconds)","seed":$seed,"term":"${TERM:-}","colorterm":"${COLORTERM:-}","no_color":"${NO_COLOR:-}","tmux":"${TMUX:-}","zellij":"${ZELLIJ:-}","kitty_window_id":"${KITTY_WINDOW_ID:-}","term_program":"${TERM_PROGRAM:-}"}
EOF
}

write_large_case_meta() {
    local jsonl="$1"
    local case_name="$2"
    local status="$3"
    local seed="$4"
    local screen_mode="$5"
    local cols="$6"
    local rows="$7"
    local ui_height="$8"
    local diff_bayes="$9"
    local bocpd="${10}"
    local conformal="${11}"
    local evidence_jsonl="${12}"
    local pty_out="${13}"
    local caps_file="${14}"
    local duration_ms="${15}"
    local run_id="${16}"

    if command -v jq >/dev/null 2>&1; then
        jq -nc \
            --arg case "$case_name" \
            --arg status "$status" \
            --arg timestamp "$(date -Iseconds)" \
            --arg run_id "$run_id" \
            --argjson seed "$seed" \
            --arg screen_mode "$screen_mode" \
            --argjson cols "$cols" \
            --argjson rows "$rows" \
            --argjson ui_height "$ui_height" \
            --argjson diff_bayesian "$diff_bayes" \
            --argjson bocpd "$bocpd" \
            --argjson conformal "$conformal" \
            --arg evidence_jsonl "$evidence_jsonl" \
            --arg pty_output "$pty_out" \
            --arg caps_file "$caps_file" \
            --argjson duration_ms "$duration_ms" \
            '{event:"large_screen_case",case:$case,status:$status,timestamp:$timestamp,run_id:$run_id,seed:$seed,screen_mode:$screen_mode,cols:$cols,rows:$rows,ui_height:$ui_height,diff_bayesian:$diff_bayesian,bocpd:$bocpd,conformal:$conformal,evidence_jsonl:$evidence_jsonl,pty_output:$pty_output,caps_file:$caps_file,duration_ms:$duration_ms}' \
            >> "$jsonl"
    else
        printf '{"event":"large_screen_case","case":"%s","status":"%s","timestamp":"%s","run_id":"%s","seed":%s,"screen_mode":"%s","cols":%s,"rows":%s,"ui_height":%s,"diff_bayesian":%s,"bocpd":%s,"conformal":%s,"evidence_jsonl":"%s","pty_output":"%s","caps_file":"%s","duration_ms":%s}\n' \
            "$(escape_json "$case_name")" "$(escape_json "$status")" "$(date -Iseconds)" "$(escape_json "$run_id")" \
            "$seed" "$(escape_json "$screen_mode")" "$cols" "$rows" "$ui_height" \
            "$diff_bayes" "$bocpd" "$conformal" \
            "$(escape_json "$evidence_jsonl")" "$(escape_json "$pty_out")" "$(escape_json "$caps_file")" \
            "$duration_ms" \
            >> "$jsonl"
    fi
}

annotate_evidence_run_id() {
    local source="$1"
    local dest="$2"
    local run_id="$3"

    awk -v run_id="$run_id" '
        {
            if ($0 ~ /"run_id":/) { print; next }
            if ($0 ~ /^\{/) { sub(/^\{/, "{\"run_id\":\"" run_id "\",", $0) }
            print
        }
    ' "$source" > "$dest"
}

check_policy_evidence() {
    local evidence_jsonl="$1"
    local case_name="$2"
    local missing=0

    if ! rg -q '"event":"diff_decision"' "$evidence_jsonl"; then
        log_test_fail "$case_name" "missing diff_decision evidence"
        missing=1
    fi
    if ! rg -q '"event":"bocpd"' "$evidence_jsonl"; then
        log_test_fail "$case_name" "missing bocpd evidence"
        missing=1
    fi
    if ! rg -q '"event":"budget_decision"' "$evidence_jsonl"; then
        log_test_fail "$case_name" "missing budget_decision evidence"
        missing=1
    fi

    return "$missing"
}

check_widget_refresh_evidence() {
    local evidence_jsonl="$1"
    local case_name="$2"
    local missing=0

    if ! rg -q '"event":"widget_refresh"' "$evidence_jsonl"; then
        log_test_fail "$case_name" "missing widget_refresh evidence"
        missing=1
    fi

    local max_skipped=0
    if command -v jq >/dev/null 2>&1; then
        max_skipped=$(jq -s '[.[] | select(.event=="widget_refresh") | .skipped_count] | max // 0' "$evidence_jsonl")
    else
        max_skipped=$(rg -o '"skipped_count":[0-9]+' "$evidence_jsonl" | awk -F: '{print $2}' | sort -n | tail -1)
        max_skipped="${max_skipped:-0}"
    fi
    if [[ "$max_skipped" -le 0 ]]; then
        log_test_fail "$case_name" "widget_refresh never dropped any widgets"
        missing=1
    fi

    return "$missing"
}

widget_refresh_hash() {
    rg '"event":"widget_refresh"' "$1" | sha256sum | awk '{print $1}'
}

write_budget_case_meta() {
    local jsonl="$1"
    local case_name="$2"
    local status="$3"
    local seed="$4"
    local screen_mode="$5"
    local cols="$6"
    local rows="$7"
    local frame_budget_us="$8"
    local render_budget_us="$9"
    local evidence_jsonl="${10}"
    local pty_out="${11}"
    local duration_ms="${12}"
    local run_id="${13}"
    local refresh_hash="${14}"

    if command -v jq >/dev/null 2>&1; then
        jq -nc \
            --arg case "$case_name" \
            --arg status "$status" \
            --arg timestamp "$(date -Iseconds)" \
            --arg run_id "$run_id" \
            --argjson seed "$seed" \
            --arg screen_mode "$screen_mode" \
            --argjson cols "$cols" \
            --argjson rows "$rows" \
            --argjson frame_budget_us "$frame_budget_us" \
            --argjson render_budget_us "$render_budget_us" \
            --arg evidence_jsonl "$evidence_jsonl" \
            --arg pty_output "$pty_out" \
            --argjson duration_ms "$duration_ms" \
            --arg refresh_hash "$refresh_hash" \
            '{event:"budgeted_refresh_case",case:$case,status:$status,timestamp:$timestamp,run_id:$run_id,seed:$seed,screen_mode:$screen_mode,cols:$cols,rows:$rows,frame_budget_us:$frame_budget_us,render_budget_us:$render_budget_us,evidence_jsonl:$evidence_jsonl,pty_output:$pty_output,duration_ms:$duration_ms,widget_refresh_hash:$refresh_hash}' \
            >> "$jsonl"
    else
        printf '{"event":"budgeted_refresh_case","case":"%s","status":"%s","timestamp":"%s","run_id":"%s","seed":%s,"screen_mode":"%s","cols":%s,"rows":%s,"frame_budget_us":%s,"render_budget_us":%s,"evidence_jsonl":"%s","pty_output":"%s","duration_ms":%s,"widget_refresh_hash":"%s"}\n' \
            "$(escape_json "$case_name")" "$(escape_json "$status")" "$(date -Iseconds)" "$(escape_json "$run_id")" \
            "$seed" "$(escape_json "$screen_mode")" "$cols" "$rows" "$frame_budget_us" "$render_budget_us" \
            "$(escape_json "$evidence_jsonl")" "$(escape_json "$pty_out")" "$duration_ms" "$(escape_json "$refresh_hash")" \
            >> "$jsonl"
    fi
}

run_large_case() {
    local case_name="$1"
    local screen_mode="$2"
    local cols="$3"
    local rows="$4"
    local ui_height="$5"
    local seed="$6"
    local diff_bayes="$7"
    local bocpd="$8"
    local conformal="$9"
    local jsonl="${10}"
    local run_id="${11}"

    LOG_FILE="$E2E_LOG_DIR/${case_name}.log"
    local output_file="$E2E_LOG_DIR/${case_name}.pty"
    local evidence_jsonl_raw="$E2E_LOG_DIR/${case_name}_evidence_raw.jsonl"
    local evidence_jsonl="$E2E_LOG_DIR/${case_name}_evidence.jsonl"
    local caps_file="$E2E_LOG_DIR/${case_name}_caps.log"

    log_test_start "$case_name"
    record_terminal_caps "$caps_file"

    local start_ms
    start_ms="$(date +%s%3N)"

    FTUI_HARNESS_SCREEN_MODE="$screen_mode" \
    FTUI_HARNESS_UI_HEIGHT="$ui_height" \
    FTUI_HARNESS_EXIT_AFTER_MS=1400 \
    FTUI_HARNESS_LOG_LINES=24 \
    FTUI_HARNESS_SUPPRESS_WELCOME=1 \
    FTUI_HARNESS_SEED="$seed" \
    FTUI_HARNESS_DIFF_BAYESIAN="$diff_bayes" \
    FTUI_HARNESS_BOCPD="$bocpd" \
    FTUI_HARNESS_CONFORMAL="$conformal" \
    FTUI_HARNESS_EVIDENCE_JSONL="$evidence_jsonl_raw" \
    PTY_COLS="$cols" \
    PTY_ROWS="$rows" \
    PTY_TIMEOUT=6 \
    PTY_CANONICALIZE=1 \
    PTY_TEST_NAME="$case_name" \
        pty_run "$output_file" "$E2E_HARNESS_BIN"

    local end_ms
    end_ms="$(date +%s%3N)"
    local duration_ms=$((end_ms - start_ms))

    local size
    size=$(wc -c < "$output_file" | tr -d ' ')
    if [[ "$size" -lt 800 ]]; then
        log_test_fail "$case_name" "insufficient PTY output ($size bytes)"
        record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "insufficient output"
        write_large_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$ui_height" "$diff_bayes" "$bocpd" "$conformal" "$evidence_jsonl" "$output_file" "$caps_file" "$duration_ms" "$run_id"
        return 1
    fi

    if [[ ! -s "$evidence_jsonl_raw" ]]; then
        log_test_fail "$case_name" "missing evidence log"
        record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "missing evidence log"
        write_large_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$ui_height" "$diff_bayes" "$bocpd" "$conformal" "$evidence_jsonl" "$output_file" "$caps_file" "$duration_ms" "$run_id"
        return 1
    fi

    annotate_evidence_run_id "$evidence_jsonl_raw" "$evidence_jsonl" "$run_id"

    if [[ "$diff_bayes" == "1" && "$bocpd" == "1" && "$conformal" == "1" ]]; then
        if ! check_policy_evidence "$evidence_jsonl" "$case_name"; then
            record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "missing policy evidence"
            write_large_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$ui_height" "$diff_bayes" "$bocpd" "$conformal" "$evidence_jsonl" "$output_file" "$caps_file" "$duration_ms" "$run_id"
            return 1
        fi
    fi

    log_test_pass "$case_name"
    record_result "$case_name" "passed" "$duration_ms" "$LOG_FILE"
    write_large_case_meta "$jsonl" "$case_name" "passed" "$seed" "$screen_mode" "$cols" "$rows" "$ui_height" "$diff_bayes" "$bocpd" "$conformal" "$evidence_jsonl" "$output_file" "$caps_file" "$duration_ms" "$run_id"
}

run_budget_case() {
    local case_name="$1"
    local screen_mode="$2"
    local cols="$3"
    local rows="$4"
    local seed="$5"
    local frame_budget_us="$6"
    local render_budget_us="$7"
    local jsonl="$8"
    local run_id="$9"

    LOG_FILE="$E2E_LOG_DIR/${case_name}.log"
    local output_file="$E2E_LOG_DIR/${case_name}.pty"
    local evidence_jsonl="$E2E_LOG_DIR/${case_name}_evidence.jsonl"

    log_test_start "$case_name"

    local start_ms
    start_ms="$(date +%s%3N)"

    FTUI_HARNESS_SCREEN_MODE="$screen_mode" \
    FTUI_HARNESS_VIEW="widget-budget" \
    FTUI_HARNESS_EXIT_AFTER_MS=1200 \
    FTUI_HARNESS_SUPPRESS_WELCOME=1 \
    FTUI_HARNESS_LOG_LINES=0 \
    FTUI_HARNESS_SEED="$seed" \
    FTUI_HARNESS_FRAME_BUDGET_US="$frame_budget_us" \
    FTUI_HARNESS_RENDER_BUDGET_US="$render_budget_us" \
    FTUI_HARNESS_EVIDENCE_JSONL="$evidence_jsonl" \
    PTY_COLS="$cols" \
    PTY_ROWS="$rows" \
    PTY_TIMEOUT=6 \
    PTY_CANONICALIZE=1 \
    PTY_TEST_NAME="$case_name" \
        pty_run "$output_file" "$E2E_HARNESS_BIN"

    local end_ms
    end_ms="$(date +%s%3N)"
    local duration_ms=$((end_ms - start_ms))

    local size
    size=$(wc -c < "$output_file" | tr -d ' ')
    if [[ "$size" -lt 800 ]]; then
        log_test_fail "$case_name" "insufficient PTY output ($size bytes)"
        record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "insufficient output"
        write_budget_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$frame_budget_us" "$render_budget_us" "$evidence_jsonl" "$output_file" "$duration_ms" "$run_id" ""
        return 1
    fi

    if [[ ! -s "$evidence_jsonl" ]]; then
        log_test_fail "$case_name" "missing evidence log"
        record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "missing evidence log"
        write_budget_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$frame_budget_us" "$render_budget_us" "$evidence_jsonl" "$output_file" "$duration_ms" "$run_id" ""
        return 1
    fi

    if ! check_widget_refresh_evidence "$evidence_jsonl" "$case_name"; then
        record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "missing widget_refresh evidence"
        write_budget_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$frame_budget_us" "$render_budget_us" "$evidence_jsonl" "$output_file" "$duration_ms" "$run_id" ""
        return 1
    fi

    local refresh_hash
    refresh_hash="$(widget_refresh_hash "$evidence_jsonl")"
    if [[ -z "$refresh_hash" ]]; then
        log_test_fail "$case_name" "empty widget_refresh hash"
        record_result "$case_name" "failed" "$duration_ms" "$LOG_FILE" "empty widget_refresh hash"
        write_budget_case_meta "$jsonl" "$case_name" "failed" "$seed" "$screen_mode" "$cols" "$rows" "$frame_budget_us" "$render_budget_us" "$evidence_jsonl" "$output_file" "$duration_ms" "$run_id" ""
        return 1
    fi

    printf '%s\n' "$refresh_hash" > "$E2E_LOG_DIR/${case_name}_widget_refresh.sha"

    log_test_pass "$case_name"
    record_result "$case_name" "passed" "$duration_ms" "$LOG_FILE"
    write_budget_case_meta "$jsonl" "$case_name" "passed" "$seed" "$screen_mode" "$cols" "$rows" "$frame_budget_us" "$render_budget_us" "$evidence_jsonl" "$output_file" "$duration_ms" "$run_id" "$refresh_hash"
}

if $RUN_LARGE; then
    log_info "Running large-screen scenarios (inline + altscreen)"

    TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
    E2E_HARNESS_BIN="${E2E_HARNESS_BIN:-$TARGET_DIR/debug/ftui-harness}"
    export E2E_HARNESS_BIN

    if [[ ! -x "$E2E_HARNESS_BIN" ]]; then
        log_warn "ftui-harness binary missing; skipping large-screen scenarios"
    else
        LARGE_JSONL="$E2E_LOG_DIR/large_screen.jsonl"
        SEED="${FTUI_HARNESS_SEED:-${E2E_SEED:-0}}"
        export FTUI_HARNESS_SEED="$SEED"

        DIFF_BAYES="${FTUI_HARNESS_DIFF_BAYESIAN:-1}"
        BOCPD="${FTUI_HARNESS_BOCPD:-1}"
        CONFORMAL="${FTUI_HARNESS_CONFORMAL:-1}"

        RUN_ID="large_screen_${TIMESTAMP}_$$"
        write_large_env "$LARGE_JSONL" "$SEED" "$RUN_ID"

        LARGE_FAILURES=0
        run_large_case "large_inline_200x60" "inline" 200 60 12 "$SEED" "$DIFF_BAYES" "$BOCPD" "$CONFORMAL" "$LARGE_JSONL" "$RUN_ID" || LARGE_FAILURES=$((LARGE_FAILURES + 1))
        run_large_case "large_inline_240x80" "inline" 240 80 12 "$SEED" "$DIFF_BAYES" "$BOCPD" "$CONFORMAL" "$LARGE_JSONL" "$RUN_ID" || LARGE_FAILURES=$((LARGE_FAILURES + 1))
        run_large_case "large_altscreen_200x60" "altscreen" 200 60 0 "$SEED" "$DIFF_BAYES" "$BOCPD" "$CONFORMAL" "$LARGE_JSONL" "$RUN_ID" || LARGE_FAILURES=$((LARGE_FAILURES + 1))
        run_large_case "large_altscreen_240x80" "altscreen" 240 80 0 "$SEED" "$DIFF_BAYES" "$BOCPD" "$CONFORMAL" "$LARGE_JSONL" "$RUN_ID" || LARGE_FAILURES=$((LARGE_FAILURES + 1))

        if [[ "$LARGE_FAILURES" -gt 0 ]]; then
            log_error "$LARGE_FAILURES large-screen scenario(s) failed"
            RUN_ALL_STATUS=1
        fi
    fi
fi

if $RUN_BUDGETED; then
    log_info "Running budgeted refresh scenario"

    TARGET_DIR="${CARGO_TARGET_DIR:-$PROJECT_ROOT/target}"
    E2E_HARNESS_BIN="${E2E_HARNESS_BIN:-$TARGET_DIR/debug/ftui-harness}"
    export E2E_HARNESS_BIN

    if [[ ! -x "$E2E_HARNESS_BIN" ]]; then
        log_test_skip "budgeted_refresh" "ftui-harness binary missing"
        record_result "budgeted_refresh" "skipped" 0 "$LOG_FILE" "missing harness binary"
    else
        BUDGET_JSONL="$E2E_LOG_DIR/budget_refresh.jsonl"
        SEED="${FTUI_HARNESS_SEED:-${E2E_SEED:-0}}"
        FRAME_BUDGET_US="${FTUI_HARNESS_FRAME_BUDGET_US:-6000}"
        RENDER_BUDGET_US="${FTUI_HARNESS_RENDER_BUDGET_US:-2500}"
        RUN_ID="budget_refresh_${TIMESTAMP}_$$"

        BUDGET_FAILURES=0
        run_budget_case "budgeted_refresh_run1" "inline" 120 40 "$SEED" "$FRAME_BUDGET_US" "$RENDER_BUDGET_US" "$BUDGET_JSONL" "$RUN_ID" || BUDGET_FAILURES=1
        run_budget_case "budgeted_refresh_run2" "inline" 120 40 "$SEED" "$FRAME_BUDGET_US" "$RENDER_BUDGET_US" "$BUDGET_JSONL" "$RUN_ID" || BUDGET_FAILURES=1

        if [[ "$BUDGET_FAILURES" -eq 0 ]]; then
            hash1=""
            hash2=""
            hash1="$(cat "$E2E_LOG_DIR/budgeted_refresh_run1_widget_refresh.sha" 2>/dev/null || true)"
            hash2="$(cat "$E2E_LOG_DIR/budgeted_refresh_run2_widget_refresh.sha" 2>/dev/null || true)"
            if [[ -z "$hash1" || -z "$hash2" || "$hash1" != "$hash2" ]]; then
                log_test_fail "budgeted_refresh_determinism" "widget_refresh evidence mismatch"
                if command -v diff >/dev/null 2>&1; then
                    diff -u \
                        <(rg '"event":"widget_refresh"' "$E2E_LOG_DIR/budgeted_refresh_run1_evidence.jsonl" || true) \
                        <(rg '"event":"widget_refresh"' "$E2E_LOG_DIR/budgeted_refresh_run2_evidence.jsonl" || true) \
                        | head -40 >> "$LOG_FILE" 2>&1 || true
                fi
                record_result "budgeted_refresh_determinism" "failed" 0 "$LOG_FILE" "widget_refresh evidence mismatch"
                RUN_ALL_STATUS=1
            else
                log_test_pass "budgeted_refresh_determinism"
                record_result "budgeted_refresh_determinism" "passed" 0 "$LOG_FILE"
            fi
        else
            RUN_ALL_STATUS=1
        fi
    fi
fi

exit "$RUN_ALL_STATUS"

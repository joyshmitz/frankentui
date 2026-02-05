#!/bin/bash
set -euo pipefail

# E2E: PTY canonicalization integration (bd-3ae1y)
# - Record real PTY fixtures via ftui-harness
# - Canonicalize fixtures via pty_canonicalize
# - Emit JSONL logs with input_id/mode/dims/hash/timing

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/../lib"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/pty.sh"

e2e_fixture_init "pty_canonicalize"

TIMESTAMP="$(e2e_log_stamp)"
E2E_LOG_DIR="${E2E_LOG_DIR:-/tmp/ftui_e2e_${E2E_RUN_ID}_${TIMESTAMP}}"
E2E_RESULTS_DIR="${E2E_RESULTS_DIR:-$E2E_LOG_DIR/results}"
LOG_FILE="${LOG_FILE:-$E2E_LOG_DIR/pty_canonicalize.log}"
E2E_JSONL_FILE="${E2E_JSONL_FILE:-$E2E_LOG_DIR/pty_canonicalize.jsonl}"
E2E_RUN_CMD="${E2E_RUN_CMD:-$0 $*}"

export E2E_LOG_DIR E2E_RESULTS_DIR LOG_FILE E2E_JSONL_FILE E2E_RUN_CMD

mkdir -p "$E2E_LOG_DIR" "$E2E_RESULTS_DIR"
jsonl_init

export PTY_CANONICALIZE=1

if [[ ! -x "${E2E_HARNESS_BIN:-}" ]]; then
    LOG_FILE="$E2E_LOG_DIR/pty_canonicalize_missing.log"
    log_test_skip "pty_canonicalize" "ftui-harness binary missing"
    record_result "pty_canonicalize" "skipped" 0 "$LOG_FILE" "binary missing"
    jsonl_run_end "skipped" 0 0
    exit 0
fi

if ! CANON_BIN="$(resolve_canonicalize_bin)"; then
    LOG_FILE="$E2E_LOG_DIR/pty_canonicalize_missing.log"
    log_test_skip "pty_canonicalize" "pty_canonicalize binary missing"
    record_result "pty_canonicalize" "skipped" 0 "$LOG_FILE" "binary missing"
    jsonl_run_end "skipped" 0 0
    exit 0
fi

emit_pty_capture() {
    local input_id="$1"
    local mode="$2"
    local cols="$3"
    local rows="$4"
    local output_file="$5"
    local canonical_file="$6"
    local duration_ms="$7"
    local status="$8"
    local exit_code="$9"

    local ts output_sha output_bytes canonical_sha canonical_bytes seed_json
    ts="$(e2e_timestamp)"
    output_sha=""
    output_bytes=0
    if [[ -f "$output_file" ]]; then
        output_sha="$(sha256_file "$output_file" || true)"
        output_bytes=$(wc -c < "$output_file" 2>/dev/null | tr -d ' ')
    fi
    canonical_sha=""
    canonical_bytes=0
    if [[ -n "$canonical_file" && -f "$canonical_file" ]]; then
        canonical_sha="$(sha256_file "$canonical_file" || true)"
        canonical_bytes=$(wc -c < "$canonical_file" 2>/dev/null | tr -d ' ')
    fi
    seed_json="null"
    if [[ -n "${E2E_SEED:-}" ]]; then seed_json="${E2E_SEED}"; fi

    if command -v jq >/dev/null 2>&1; then
        jsonl_emit "$(jq -nc \
            --arg schema_version "$E2E_JSONL_SCHEMA_VERSION" \
            --arg type "pty_capture" \
            --arg timestamp "$ts" \
            --arg run_id "$E2E_RUN_ID" \
            --arg input_id "$input_id" \
            --arg mode "$mode" \
            --arg status "$status" \
            --arg output_file "$output_file" \
            --arg canonical_file "$canonical_file" \
            --arg output_sha256 "$output_sha" \
            --arg canonical_sha256 "$canonical_sha" \
            --argjson duration_ms "$duration_ms" \
            --argjson output_bytes "${output_bytes:-0}" \
            --argjson canonical_bytes "${canonical_bytes:-0}" \
            --argjson cols "$cols" \
            --argjson rows "$rows" \
            --argjson exit_code "$exit_code" \
            --argjson seed "$seed_json" \
            '{schema_version:$schema_version,type:$type,timestamp:$timestamp,run_id:$run_id,seed:$seed,input_id:$input_id,mode:$mode,status:$status,duration_ms:$duration_ms,output_file:$output_file,canonical_file:$canonical_file,output_sha256:$output_sha256,canonical_sha256:$canonical_sha256,output_bytes:$output_bytes,canonical_bytes:$canonical_bytes,cols:$cols,rows:$rows,exit_code:$exit_code}')"
    else
        jsonl_emit "{\"schema_version\":\"${E2E_JSONL_SCHEMA_VERSION}\",\"type\":\"pty_capture\",\"timestamp\":\"$(json_escape "$ts")\",\"run_id\":\"$(json_escape "$E2E_RUN_ID")\",\"seed\":${seed_json},\"input_id\":\"$(json_escape "$input_id")\",\"mode\":\"$(json_escape "$mode")\",\"status\":\"$(json_escape "$status")\",\"duration_ms\":${duration_ms},\"output_file\":\"$(json_escape "$output_file")\",\"canonical_file\":\"$(json_escape "$canonical_file")\",\"output_sha256\":\"$(json_escape "$output_sha")\",\"canonical_sha256\":\"$(json_escape "$canonical_sha")\",\"output_bytes\":${output_bytes:-0},\"canonical_bytes\":${canonical_bytes:-0},\"cols\":${cols},\"rows\":${rows},\"exit_code\":${exit_code}}"
    fi
}

run_fixture() {
    local input_id="$1"
    local mode="$2"
    local cols="$3"
    local rows="$4"
    local ui_height="$5"

    local output_file="$E2E_LOG_DIR/${input_id}.pty"
    local canonical_file="$E2E_LOG_DIR/${input_id}.canonical.txt"
    local repeat_file="$E2E_LOG_DIR/${input_id}.canonical.repeat.txt"

    LOG_FILE="$E2E_LOG_DIR/${input_id}.log"
    log_test_start "$input_id"

    export E2E_CONTEXT_MODE="$mode"
    export E2E_CONTEXT_COLS="$cols"
    export E2E_CONTEXT_ROWS="$rows"
    export E2E_CONTEXT_SEED="${E2E_SEED:-0}"
    jsonl_step_start "$input_id"

    local start_ms
    start_ms="$(e2e_now_ms)"

    local run_exit=0
    if PTY_COLS="$cols" \
        PTY_ROWS="$rows" \
        PTY_TIMEOUT=4 \
        PTY_CANONICALIZE=0 \
        PTY_TEST_NAME="$input_id" \
        FTUI_HARNESS_EXIT_AFTER_MS=900 \
        FTUI_HARNESS_LOG_LINES=6 \
        FTUI_HARNESS_SUPPRESS_WELCOME=1 \
        FTUI_HARNESS_SCREEN_MODE="$mode" \
        FTUI_HARNESS_UI_HEIGHT="$ui_height" \
            pty_run "$output_file" "$E2E_HARNESS_BIN"; then
        run_exit=0
    else
        run_exit=$?
    fi
    local end_ms
    end_ms="$(e2e_now_ms)"
    local duration_ms=$((end_ms - start_ms))

    if [[ "$run_exit" -ne 0 || ! -s "$output_file" ]]; then
        log_test_fail "$input_id" "pty capture failed"
        record_result "$input_id" "failed" "$duration_ms" "$LOG_FILE" "pty capture failed"
        jsonl_step_end "$input_id" "failed" "$duration_ms"
        emit_pty_capture "$input_id" "$mode" "$cols" "$rows" "$output_file" "" "$duration_ms" "failed" "$run_exit"
        jsonl_assert "pty_capture_${input_id}" "failed" "pty capture failed"
        return 1
    fi

    local canon_start
    canon_start="$(e2e_now_ms)"
    if ! "$CANON_BIN" --input "$output_file" --output "$canonical_file" --cols "$cols" --rows "$rows"; then
        local canon_end
        canon_end="$(e2e_now_ms)"
        local canon_ms=$((canon_end - canon_start))
        log_test_fail "$input_id" "pty_canonicalize failed"
        record_result "$input_id" "failed" "$canon_ms" "$LOG_FILE" "pty_canonicalize failed"
        jsonl_step_end "$input_id" "failed" "$canon_ms"
        emit_pty_capture "$input_id" "$mode" "$cols" "$rows" "$output_file" "" "$canon_ms" "failed" 1
        jsonl_assert "pty_canonicalize_${input_id}" "failed" "pty_canonicalize failed"
        return 1
    fi

    local canon_end
    canon_end="$(e2e_now_ms)"
    local canon_ms=$((canon_end - canon_start))

    if e2e_is_deterministic; then
        if "$CANON_BIN" --input "$output_file" --output "$repeat_file" --cols "$cols" --rows "$rows"; then
            local canon_sha repeat_sha
            canon_sha="$(sha256_file "$canonical_file" || true)"
            repeat_sha="$(sha256_file "$repeat_file" || true)"
            if [[ -n "$canon_sha" && "$canon_sha" != "$repeat_sha" ]]; then
                log_test_fail "$input_id" "canonical hash mismatch"
                record_result "$input_id" "failed" "$canon_ms" "$LOG_FILE" "canonical hash mismatch"
                jsonl_assert "pty_hash_stability_${input_id}" "failed" "expected ${canon_sha}, got ${repeat_sha}"
                jsonl_step_end "$input_id" "failed" "$canon_ms"
                emit_pty_capture "$input_id" "$mode" "$cols" "$rows" "$output_file" "$canonical_file" "$canon_ms" "failed" 0
                return 1
            fi
            jsonl_assert "pty_hash_stability_${input_id}" "passed" "canonical hash stable"
        else
            jsonl_assert "pty_hash_stability_${input_id}" "failed" "repeat canonicalize failed"
            jsonl_step_end "$input_id" "failed" "$canon_ms"
            emit_pty_capture "$input_id" "$mode" "$cols" "$rows" "$output_file" "$canonical_file" "$canon_ms" "failed" 1
            return 1
        fi
    fi

    log_test_pass "$input_id"
    record_result "$input_id" "passed" "$canon_ms" "$LOG_FILE"
    jsonl_step_end "$input_id" "passed" "$canon_ms"
    emit_pty_capture "$input_id" "$mode" "$cols" "$rows" "$output_file" "$canonical_file" "$canon_ms" "passed" 0
    return 0
}

FAILURES=0

run_fixture "pty_inline_smoke" "inline" 80 24 8 || FAILURES=$((FAILURES + 1))
run_fixture "pty_altscreen_smoke" "altscreen" 100 30 10 || FAILURES=$((FAILURES + 1))

run_end_ms="$(e2e_now_ms)"
run_duration_ms=$((run_end_ms - ${E2E_RUN_START_MS:-0}))
if [[ "$FAILURES" -eq 0 ]]; then
    jsonl_run_end "passed" "$run_duration_ms" 0
else
    jsonl_run_end "failed" "$run_duration_ms" "$FAILURES"
fi

exit "$FAILURES"

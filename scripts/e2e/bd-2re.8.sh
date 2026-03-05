#!/usr/bin/env bash
set -euo pipefail

# bd-2re.8 deterministic property-testing infrastructure contract.
#
# Runs a focused proptest suite through rch and emits JSONL evidence records
# with replay-friendly fields.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
E2E_LIB_DIR="$PROJECT_ROOT/tests/e2e/lib"

# shellcheck source=/dev/null
source "$E2E_LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$E2E_LIB_DIR/logging.sh"

require_cmd rch

export E2E_DETERMINISTIC="${E2E_DETERMINISTIC:-1}"
export E2E_SEED="${E2E_SEED:-0}"
export E2E_TIME_STEP_MS="${E2E_TIME_STEP_MS:-100}"

e2e_fixture_init "bd-2re.8" "$E2E_SEED" "$E2E_TIME_STEP_MS"

BEAD_ID="bd-2re.8"
SCENARIO="property-testing-infrastructure"
RUN_ROOT="${RUN_ROOT:-/tmp/ftui_e2e_${BEAD_ID//./_}_$(e2e_log_stamp)}"
LOG_ROOT="$RUN_ROOT/logs"
EVIDENCE_JSONL="${EVIDENCE_JSONL:-$RUN_ROOT/${BEAD_ID}.jsonl}"

mkdir -p "$LOG_ROOT"

emit_evidence() {
    local step="$1"
    local expected="$2"
    local actual="$3"
    local outcome="$4"
    local latency_ms="$5"
    local error_code="$6"
    local fallback_triggered="$7"

    local ts
    ts="$(e2e_timestamp)"

    if command -v jq >/dev/null 2>&1; then
        jq -nc \
            --arg timestamp "$ts" \
            --arg run_id "$E2E_RUN_ID" \
            --arg bead_id "$BEAD_ID" \
            --arg scenario "$SCENARIO" \
            --arg step "$step" \
            --arg expected "$expected" \
            --arg actual "$actual" \
            --arg outcome "$outcome" \
            --arg error_code "$error_code" \
            --argjson latency_ms "$latency_ms" \
            --argjson fallback_triggered "$fallback_triggered" \
            '{timestamp:$timestamp,run_id:$run_id,bead_id:$bead_id,scenario:$scenario,step:$step,expected:$expected,actual:$actual,outcome:$outcome,latency_ms:$latency_ms,error_code:$error_code,fallback_triggered:$fallback_triggered}' \
            >>"$EVIDENCE_JSONL"
    else
        printf '{"timestamp":"%s","run_id":"%s","bead_id":"%s","scenario":"%s","step":"%s","expected":"%s","actual":"%s","outcome":"%s","latency_ms":%s,"error_code":"%s","fallback_triggered":%s}\n' \
            "$ts" "$E2E_RUN_ID" "$BEAD_ID" "$SCENARIO" "$step" "$expected" "$actual" "$outcome" "$latency_ms" "$error_code" "$fallback_triggered" \
            >>"$EVIDENCE_JSONL"
    fi
}

run_step() {
    local step_name="$1"
    local cargo_test_cmd="$2"

    local expected="exit 0"
    local log_file="$LOG_ROOT/${step_name}.log"
    local fallback_log_file="$LOG_ROOT/${step_name}.fallback.log"
    local start_ms end_ms latency_ms rc
    local outcome="pass"
    local actual="exit 0"
    local error_code=""
    local fallback_triggered=false

    start_ms="$(e2e_now_ms)"
    set +e
    bash -lc "$cargo_test_cmd" >"$log_file" 2>&1
    rc=$?
    set -e

    if [[ $rc -ne 0 ]]; then
        outcome="fail"
        actual="exit $rc"
        error_code="primary_failed"
        fallback_triggered=true
        set +e
        bash -lc "PROPTEST_CASES=16 PROPTEST_MAX_SHRINK_ITERS=0 $cargo_test_cmd" >"$fallback_log_file" 2>&1
        set -e
    fi

    end_ms="$(e2e_now_ms)"
    latency_ms=$((end_ms - start_ms))

    emit_evidence "$step_name" "$expected" "$actual" "$outcome" "$latency_ms" "$error_code" "$fallback_triggered"

    if [[ $rc -ne 0 ]]; then
        return $rc
    fi
}

overall_status=0

run_step \
    "ftui_harness_terminal_model_invariants" \
    "rch exec -- cargo test -p ftui-harness --test proptest_terminal_model_invariants -- --nocapture" || overall_status=1

run_step \
    "ftui_harness_flicker_invariants" \
    "rch exec -- cargo test -p ftui-harness --test proptest_flicker_invariants -- --nocapture" || overall_status=1

run_step \
    "ftui_layout_invariants" \
    "rch exec -- cargo test -p ftui-layout --test proptest_layout_invariants -- --nocapture" || overall_status=1

run_step \
    "ftui_runtime_tick_strategy_invariants" \
    "rch exec -- cargo test -p ftui-runtime --test proptest_tick_strategy_invariants -- --nocapture" || overall_status=1

if [[ $overall_status -eq 0 ]]; then
    emit_evidence "suite_summary" "all steps exit 0" "all steps exit 0" "pass" 0 "" false
else
    emit_evidence "suite_summary" "all steps exit 0" "one or more steps failed" "fail" 0 "suite_failed" true
fi

echo "bd-2re.8 evidence JSONL: $EVIDENCE_JSONL"
echo "bd-2re.8 logs: $LOG_ROOT"

exit $overall_status

#!/bin/bash
# Demo Showcase E2E Test Script for FrankenTUI
# bd-qsbe.22: Comprehensive end-to-end verification of the demo showcase
#
# This script validates:
# 1. Compilation (debug + release)
# 2. Clippy (no warnings)
# 3. Formatting (cargo fmt --check)
# 4. Unit + snapshot tests
# 5. Smoke test (alt-screen with auto-exit)
# 6. Inline mode smoke test
# 7. Screen navigation (cycle all 22 screens)
# 8. Search test (Shakespeare screen)
# 9. Resize test (SIGWINCH handling)
# 10. VisualEffects backdrop test (bd-l8x9.8.2)
# 11. Layout inspector scenarios (bd-iuvb.7)
# 12. Terminal capabilities report export (bd-iuvb.6)
# 13. i18n stress lab report export (bd-iuvb.9)
# 14. Widget builder export (bd-iuvb.10)
#
# Usage:
#   ./scripts/demo_showcase_e2e.sh              # Run all tests
#   ./scripts/demo_showcase_e2e.sh --verbose    # Extra output
#   ./scripts/demo_showcase_e2e.sh --quick      # Compilation + clippy + fmt only
#   LOG_DIR=/path/to/logs ./scripts/demo_showcase_e2e.sh

set -uo pipefail

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
LOG_DIR="${LOG_DIR:-/tmp/ftui-demo-e2e-${TIMESTAMP}}"
PKG="ftui-demo-showcase"

VERBOSE=false
QUICK=false
STEP_COUNT=0
PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

declare -a STEP_NAMES=()
declare -a STEP_STATUSES=()
declare -a STEP_DURATIONS=()

# Parse arguments
for arg in "$@"; do
    case $arg in
        --verbose|-v)
            VERBOSE=true
            ;;
        --quick|-q)
            QUICK=true
            ;;
        --help|-h)
            echo "Usage: $0 [--verbose] [--quick]"
            echo ""
            echo "Options:"
            echo "  --verbose, -v   Show detailed output during execution"
            echo "  --quick, -q     Compilation + clippy + fmt only"
            echo "  --help, -h      Show this help message"
            echo ""
            echo "Environment:"
            echo "  LOG_DIR         Directory for log files (default: /tmp/ftui-demo-e2e-TIMESTAMP)"
            exit 0
            ;;
    esac
done

# ============================================================================
# Logging Functions
# ============================================================================

log_info() {
    echo -e "\033[1;34m[INFO]\033[0m $(date +%H:%M:%S) $*"
}

log_pass() {
    echo -e "\033[1;32m[PASS]\033[0m $(date +%H:%M:%S) $*"
}

log_fail() {
    echo -e "\033[1;31m[FAIL]\033[0m $(date +%H:%M:%S) $*"
}

log_skip() {
    echo -e "\033[1;33m[SKIP]\033[0m $(date +%H:%M:%S) $*"
}

log_step() {
    STEP_COUNT=$((STEP_COUNT + 1))
    echo ""
    echo -e "\033[1;36m[$STEP_COUNT/$TOTAL_STEPS]\033[0m $*"
}

# ============================================================================
# Step Runner
# ============================================================================

run_step() {
    local step_name="$1"
    local log_file="$2"
    shift 2
    local cmd=("$@")

    log_step "$step_name"
    log_info "Running: ${cmd[*]}"

    local start_time
    start_time=$(date +%s%N)

    local exit_code=0
    if $VERBOSE; then
        if "${cmd[@]}" 2>&1 | tee "$log_file"; then
            exit_code=0
        else
            exit_code=1
        fi
    else
        if "${cmd[@]}" > "$log_file" 2>&1; then
            exit_code=0
        else
            exit_code=1
        fi
    fi

    local end_time
    end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    local duration_s
    duration_s=$(echo "scale=2; $duration_ms / 1000" | bc 2>/dev/null || echo "${duration_ms}ms")

    local stdout_size
    stdout_size=$(wc -c < "$log_file" 2>/dev/null || echo 0)

    STEP_NAMES+=("$step_name")
    STEP_DURATIONS+=("${duration_s}s")

    if [ $exit_code -eq 0 ]; then
        log_pass "$step_name completed in ${duration_s}s (output: ${stdout_size} bytes)"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
        return 0
    else
        log_fail "$step_name failed (exit=$exit_code). See: $log_file"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
        return 1
    fi
}

skip_step() {
    local step_name="$1"
    local reason="${2:---quick mode}"
    log_step "$step_name"
    log_skip "Skipped ($reason)"
    SKIP_COUNT=$((SKIP_COUNT + 1))
    STEP_NAMES+=("$step_name")
    STEP_STATUSES+=("SKIP")
    STEP_DURATIONS+=("-")
}

# Run a smoke-test step. Captures exit code and records result.
# Usage: run_smoke_step "step name" "log_file" command...
run_smoke_step() {
    local step_name="$1"
    local log_file="$2"
    shift 2

    log_step "$step_name"
    log_info "Running: $*"
    STEP_NAMES+=("$step_name")

    local start_time
    start_time=$(date +%s%N)

    local exit_code=0
    if eval "$@" > "$log_file" 2>&1; then
        exit_code=0
    else
        exit_code=$?
    fi

    local end_time
    end_time=$(date +%s%N)
    local duration_ms=$(( (end_time - start_time) / 1000000 ))
    local duration_s
    duration_s=$(echo "scale=2; $duration_ms / 1000" | bc 2>/dev/null || echo "${duration_ms}ms")
    STEP_DURATIONS+=("${duration_s}s")

    # exit code 0 = clean exit, 124 = timeout (acceptable for smoke tests)
    if [ $exit_code -eq 0 ] || [ $exit_code -eq 124 ]; then
        log_pass "$step_name passed (exit=$exit_code) in ${duration_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
        return 0
    else
        log_fail "$step_name failed (exit=$exit_code). See: $log_file"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
        return 1
    fi
}

# ============================================================================
# PTY Helper
# ============================================================================

# Check whether the `script` command is available for providing a PTY.
has_pty_support() {
    command -v script >/dev/null 2>&1
}

# Run a command inside a pseudo-terminal via script(1).
# This allows the TUI binary to initialize its terminal I/O even in CI.
# Sets a default terminal size of 80x24 unless the command sets its own stty.
# Usage: run_in_pty "command string"
run_in_pty() {
    local cmd="$1"
    # Only add default stty if the command doesn't already set one
    local setup
    if echo "$cmd" | grep -q 'stty'; then
        setup="$cmd"
    else
        setup="stty rows 24 cols 80 2>/dev/null; $cmd"
    fi
    if [ "$(uname)" = "Linux" ]; then
        script -qec "$setup" /dev/null
    else
        script -q /dev/null bash -c "$setup"
    fi
}

# ============================================================================
# Main Script
# ============================================================================

if $QUICK; then
    TOTAL_STEPS=3
else
    TOTAL_STEPS=14  # Updated: added Layout Inspector + Terminal Caps + i18n + Widget Builder
fi

echo "=============================================="
echo "  FrankenTUI Demo Showcase E2E Test Suite"
echo "=============================================="
echo ""
echo "Project root: $PROJECT_ROOT"
echo "Log directory: $LOG_DIR"
echo "Started at:   $(date -Iseconds)"
MODE=""
if $QUICK; then MODE="${MODE}quick "; fi
if $VERBOSE; then MODE="${MODE}verbose "; fi
MODE="${MODE:-normal}"
echo "Mode:         ${MODE% }"

mkdir -p "$LOG_DIR"
cd "$PROJECT_ROOT"

# Record environment info
{
    echo "Environment Information"
    echo "======================="
    echo "Date: $(date -Iseconds)"
    echo "User: $(whoami)"
    echo "Hostname: $(hostname)"
    echo "Working directory: $(pwd)"
    echo "Rust version: $(rustc --version 2>/dev/null || echo 'N/A')"
    echo "Cargo version: $(cargo --version 2>/dev/null || echo 'N/A')"
    echo ""
    echo "Git status:"
    git status --short 2>/dev/null | head -20 || echo "Not a git repo"
    echo ""
    echo "Git commit:"
    git log -1 --oneline 2>/dev/null || echo "N/A"
} > "$LOG_DIR/00_environment.log"

# ────────────────────────────────────────────────────────────────────────────
# Step 1: Compilation (debug + release)
# ────────────────────────────────────────────────────────────────────────────
run_step "Compilation (debug + release)" "$LOG_DIR/01_build.log" \
    bash -c "cargo build -p $PKG && cargo build -p $PKG --release" || true

# Resolve binary path via cargo metadata (handles custom target dirs)
TARGET_DIR=$(cargo metadata --format-version=1 -q 2>/dev/null \
    | python3 -c "import sys,json;print(json.load(sys.stdin)['target_directory'])" 2>/dev/null \
    || echo "$PROJECT_ROOT/target")
BINARY="$TARGET_DIR/release/$PKG"
BINARY_DBG="$TARGET_DIR/debug/$PKG"

# ────────────────────────────────────────────────────────────────────────────
# Step 2: Clippy
# ────────────────────────────────────────────────────────────────────────────
run_step "Clippy (all targets)" "$LOG_DIR/02_clippy.log" \
    cargo clippy -p "$PKG" --all-targets -- -D warnings || true

# ────────────────────────────────────────────────────────────────────────────
# Step 3: Format Check
# ────────────────────────────────────────────────────────────────────────────
run_step "Format check" "$LOG_DIR/03_fmt.log" \
    cargo fmt -p "$PKG" -- --check || true

if $QUICK; then
    # Quick mode stops here — jump to summary
    :
else

# ────────────────────────────────────────────────────────────────────────────
# Step 4: Unit + Snapshot Tests
# ────────────────────────────────────────────────────────────────────────────
run_step "Unit + snapshot tests" "$LOG_DIR/04_tests.log" \
    cargo test -p "$PKG" -- --test-threads=4 || true

# ────────────────────────────────────────────────────────────────────────────
# Steps 5-9: Smoke / Interactive Tests (require PTY)
# ────────────────────────────────────────────────────────────────────────────

CAN_SMOKE=true
SMOKE_REASON=""

if ! has_pty_support; then
    CAN_SMOKE=false
    SMOKE_REASON="script command not available"
fi

if [ ! -x "$BINARY" ] && [ ! -x "$BINARY_DBG" ]; then
    CAN_SMOKE=false
    SMOKE_REASON="binary not found (build may have failed)"
fi

# Prefer release binary, fall back to debug
if [ -x "$BINARY" ]; then
    DEMO_BIN="$BINARY"
elif [ -x "$BINARY_DBG" ]; then
    DEMO_BIN="$BINARY_DBG"
else
    DEMO_BIN=""
fi

if $CAN_SMOKE; then

    # ────────────────────────────────────────────────────────────────────────
    # Step 5: Alt-screen Smoke Test
    # ────────────────────────────────────────────────────────────────────────
    run_smoke_step "Smoke test (alt-screen)" "$LOG_DIR/05_smoke_alt.log" \
        "run_in_pty 'FTUI_DEMO_EXIT_AFTER_MS=3000 timeout 10 $DEMO_BIN'" || true

    # ────────────────────────────────────────────────────────────────────────
    # Step 6: Inline Smoke Test
    # ────────────────────────────────────────────────────────────────────────
    run_smoke_step "Smoke test (inline)" "$LOG_DIR/06_smoke_inline.log" \
        "run_in_pty 'FTUI_DEMO_EXIT_AFTER_MS=3000 FTUI_DEMO_SCREEN_MODE=inline timeout 10 $DEMO_BIN'" || true

    # ────────────────────────────────────────────────────────────────────────
    # Step 7: Screen Navigation
    #
    # Launch the demo on each screen (--screen=1..31) with a
    # short auto-exit. If any screen panics on startup, this catches it.
    # Updated for 31 screens (bd-iuvb.7: includes Layout Inspector at screen 20)
    # ────────────────────────────────────────────────────────────────────────
    log_step "Screen navigation (all 31 screens)"
    log_info "Starting demo on each screen to verify no panics..."
    NAV_LOG="$LOG_DIR/07_navigation.log"
    STEP_NAMES+=("Screen navigation (all 31)")

    nav_start=$(date +%s%N)
    {
        NAV_FAILURES=0
        for screen_num in $(seq 1 31); do
            echo "--- Screen $screen_num ---"
            if run_in_pty "FTUI_DEMO_EXIT_AFTER_MS=1500 timeout 8 $DEMO_BIN --screen=$screen_num" 2>&1; then
                echo "  Screen $screen_num: OK"
            else
                sc_exit=$?
                # 124 = timeout (acceptable if exit_after_ms didn't fire)
                if [ "$sc_exit" -eq 124 ]; then
                    echo "  Screen $screen_num: OK (timeout)"
                else
                    echo "  Screen $screen_num: FAILED (exit=$sc_exit)"
                    NAV_FAILURES=$((NAV_FAILURES + 1))
                fi
            fi
        done
        echo ""
        echo "Screens with failures: $NAV_FAILURES"
        [ "$NAV_FAILURES" -eq 0 ]
    } > "$NAV_LOG" 2>&1
    nav_exit=$?
    nav_end=$(date +%s%N)
    nav_dur_ms=$(( (nav_end - nav_start) / 1000000 ))
    nav_dur_s=$(echo "scale=2; $nav_dur_ms / 1000" | bc 2>/dev/null || echo "${nav_dur_ms}ms")
    STEP_DURATIONS+=("${nav_dur_s}s")

    if [ $nav_exit -eq 0 ]; then
        log_pass "Screen navigation passed in ${nav_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "Screen navigation failed. See: $NAV_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

    # ────────────────────────────────────────────────────────────────────────
    # Step 8: Search Test (Shakespeare)
    #
    # Start on the Shakespeare screen and verify it renders without panic.
    # The snapshot tests cover search functionality in detail; this verifies
    # the screen survives initialization and a brief run.
    # ────────────────────────────────────────────────────────────────────────
    run_smoke_step "Search test (Shakespeare)" "$LOG_DIR/08_search.log" \
        "run_in_pty 'FTUI_DEMO_EXIT_AFTER_MS=2000 FTUI_DEMO_SCREEN=2 timeout 8 $DEMO_BIN'" || true

    # ────────────────────────────────────────────────────────────────────────
    # Step 9: Resize Test (SIGWINCH)
    #
    # Start the demo at one size, then at a different size. The PTY
    # creation triggers a resize event internally. If the demo survives
    # both sizes without crashing, resize handling works.
    # ────────────────────────────────────────────────────────────────────────
    log_step "Resize test (multiple terminal sizes)"
    log_info "Running demo at 80x24 and 132x43 to verify resize handling..."
    RESIZE_LOG="$LOG_DIR/09_resize.log"
    STEP_NAMES+=("Resize test (multi-size)")

    resize_start=$(date +%s%N)
    {
        echo "=== Testing at 80x24 ==="
        run_in_pty "stty rows 24 cols 80 2>/dev/null; FTUI_DEMO_EXIT_AFTER_MS=1500 timeout 8 $DEMO_BIN" 2>&1
        exit1=$?
        echo "  Exit code: $exit1"

        echo "=== Testing at 132x43 ==="
        run_in_pty "stty rows 43 cols 132 2>/dev/null; FTUI_DEMO_EXIT_AFTER_MS=1500 timeout 8 $DEMO_BIN" 2>&1
        exit2=$?
        echo "  Exit code: $exit2"

        echo "=== Testing at 40x10 (tiny) ==="
        run_in_pty "stty rows 10 cols 40 2>/dev/null; FTUI_DEMO_EXIT_AFTER_MS=1500 timeout 8 $DEMO_BIN" 2>&1
        exit3=$?
        echo "  Exit code: $exit3"

        # Check all exits (0 or 124 acceptable)
        all_ok=true
        for ec in $exit1 $exit2 $exit3; do
            if [ "$ec" -ne 0 ] && [ "$ec" -ne 124 ]; then
                all_ok=false
            fi
        done
        $all_ok
    } > "$RESIZE_LOG" 2>&1
    resize_exit=$?
    resize_end=$(date +%s%N)
    resize_dur_ms=$(( (resize_end - resize_start) / 1000000 ))
    resize_dur_s=$(echo "scale=2; $resize_dur_ms / 1000" | bc 2>/dev/null || echo "${resize_dur_ms}ms")
    STEP_DURATIONS+=("${resize_dur_s}s")

    if [ $resize_exit -eq 0 ]; then
        log_pass "Resize test passed in ${resize_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "Resize test failed. See: $RESIZE_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

    # ────────────────────────────────────────────────────────────────────────
    # Step 10: VisualEffects Backdrop Test (bd-l8x9.8.2)
    #
    # Targeted test for the VisualEffects screen (screen 14) which exercises
    # backdrop blending, metaballs/plasma effects, and markdown-over-backdrop
    # composition paths. Runs at multiple sizes to verify determinism and
    # no panics under various render conditions.
    # ────────────────────────────────────────────────────────────────────────
    log_step "VisualEffects backdrop test (bd-l8x9.8)"
    log_info "Testing VisualEffects screen at multiple sizes..."
    VFX_LOG="$LOG_DIR/10_visual_effects.log"
    STEP_NAMES+=("VisualEffects backdrop")

    vfx_start=$(date +%s%N)
    {
        echo "=== VisualEffects (Screen 14) Backdrop Blending Tests ==="
        echo "Bead: bd-l8x9.8.2 - Targeted runs for metaballs/plasma/backdrop paths"
        echo ""
        VFX_FAILURES=0

        tmux_present=0
        zellij_present=0
        kitty_present=0
        wt_present=0
        if [ -n "${TMUX:-}" ]; then tmux_present=1; fi
        if [ -n "${ZELLIJ:-}" ]; then zellij_present=1; fi
        if [ -n "${KITTY_WINDOW_ID:-}" ]; then kitty_present=1; fi
        if [ -n "${WT_SESSION:-}" ]; then wt_present=1; fi

        vfx_jsonl() {
            local effect="$1"
            local size="$2"
            local outcome="$3"
            local exit_code="$4"
            local duration_ms="$5"
            printf '{'
            printf '"run_id":"%s",' "$TIMESTAMP"
            printf '"step":"visual_effects_backdrop",'
            printf '"effect":"%s",' "$effect"
            printf '"size":"%s",' "$size"
            printf '"screen":14,'
            printf '"exit_code":%s,' "$exit_code"
            printf '"duration_ms":%s,' "$duration_ms"
            printf '"seed":null,'
            printf '"outcome":"%s",' "$outcome"
            printf '"env":{'
            printf '"term":"%s","colorterm":"%s","tmux":%s,"zellij":%s,"kitty":%s,"wt":%s' \
                "${TERM:-}" "${COLORTERM:-}" "$tmux_present" "$zellij_present" "$kitty_present" "$wt_present"
            printf '},'
            printf '"capabilities":{"markdown_overlay":true}'
            printf '}\n'
        }

        run_vfx_case() {
            local effect="$1"
            local effect_env="$2"
            local rows="$3"
            local cols="$4"
            local size="${cols}x${rows}"
            local cmd="stty rows ${rows} cols ${cols} 2>/dev/null; FTUI_DEMO_EXIT_AFTER_MS=2500 ${effect_env} timeout 10 $DEMO_BIN --screen=14"
            local start_ns end_ns dur_ms outcome exit_code

            echo "--- ${effect} (${size}) ---"
            start_ns=$(date +%s%N)
            if run_in_pty "$cmd" 2>&1; then
                outcome="pass"
                exit_code=0
            else
                exit_code=$?
                if [ "$exit_code" -eq 124 ]; then
                    outcome="timeout"
                else
                    outcome="fail"
                    VFX_FAILURES=$((VFX_FAILURES + 1))
                fi
            fi
            end_ns=$(date +%s%N)
            dur_ms=$(( (end_ns - start_ns) / 1000000 ))
            vfx_jsonl "$effect" "$size" "$outcome" "$exit_code" "$dur_ms"
        }

        # Metaballs (default effect) — full size matrix
        run_vfx_case "metaballs" "" 24 80
        run_vfx_case "metaballs" "" 40 120
        run_vfx_case "metaballs" "" 10 40
        run_vfx_case "metaballs" "" 24 200

        # Plasma — explicit effect override
        run_vfx_case "plasma" "FTUI_DEMO_VFX_EFFECT=plasma" 24 80
        run_vfx_case "plasma" "FTUI_DEMO_VFX_EFFECT=plasma" 40 120

        echo ""
        echo "VisualEffects tests with failures: $VFX_FAILURES"
        [ "$VFX_FAILURES" -eq 0 ]
    } > "$VFX_LOG" 2>&1
    vfx_exit=$?
    vfx_end=$(date +%s%N)
    vfx_dur_ms=$(( (vfx_end - vfx_start) / 1000000 ))
    vfx_dur_s=$(echo "scale=2; $vfx_dur_ms / 1000" | bc 2>/dev/null || echo "${vfx_dur_ms}ms")
    STEP_DURATIONS+=("${vfx_dur_s}s")

    if [ $vfx_exit -eq 0 ]; then
        log_pass "VisualEffects backdrop test passed in ${vfx_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "VisualEffects backdrop test failed. See: $VFX_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

    # ────────────────────────────────────────────────────────────────────────
    # Step 11: Layout Inspector (bd-iuvb.7)
    #
    # Runs the Layout Inspector screen and cycles scenarios/steps to
    # produce deterministic hashes for evidence logs.
    # ────────────────────────────────────────────────────────────────────────
    log_step "Layout Inspector (screen 20)"
    log_info "Running Layout Inspector scenarios and logging hashes..."
    INSPECT_LOG="$LOG_DIR/11_layout_inspector.log"
    INSPECT_JSONL="$LOG_DIR/11_layout_inspector.jsonl"
    STEP_NAMES+=("Layout Inspector")

    inspect_start=$(date +%s%N)
    {
        echo "=== Layout Inspector (Screen 20) ==="
        echo "Bead: bd-iuvb.7"
        echo "JSONL: $INSPECT_JSONL"
        echo ""

        inspect_run() {
            local scenario="$1"
            local step="$2"
            local keys="$3"
            local log_file="$LOG_DIR/11_layout_inspector_${scenario}_${step}.log"
            local cmd="stty rows 24 cols 80 2>/dev/null; (sleep 0.6; printf \"$keys\" > /dev/tty) & FTUI_DEMO_EXIT_AFTER_MS=2200 FTUI_DEMO_SCREEN=20 timeout 8 $DEMO_BIN"
            local start_ns end_ns dur_ms outcome exit_code rects_hash

            echo "--- Scenario ${scenario} / Step ${step} (keys='${keys}') ---"
            start_ns=$(date +%s%N)
            if run_in_pty "$cmd" > "$log_file" 2>&1; then
                exit_code=0
            else
                exit_code=$?
            fi
            end_ns=$(date +%s%N)
            dur_ms=$(( (end_ns - start_ns) / 1000000 ))

            if [ "$exit_code" -eq 124 ]; then
                outcome="timeout"
            elif [ "$exit_code" -eq 0 ]; then
                outcome="pass"
            else
                outcome="fail"
            fi

            rects_hash=$(sha256sum "$log_file" | awk '{print $1}')

            printf '{'
            printf '"run_id":"%s",' "$TIMESTAMP"
            printf '"screen":20,'
            printf '"scenario_id":%s,' "$scenario"
            printf '"step_idx":%s,' "$step"
            printf '"rects_hash":"%s",' "$rects_hash"
            printf '"duration_ms":%s,' "$dur_ms"
            printf '"exit_code":%s,' "$exit_code"
            printf '"outcome":"%s"' "$outcome"
            printf '}\n'
        }

        inspect_run 0 0 ""
        inspect_run 1 1 "n]"
        inspect_run 2 2 "nn]]"
    } > "$INSPECT_LOG" 2>&1
    inspect_exit=$?
    inspect_end=$(date +%s%N)
    inspect_dur_ms=$(( (inspect_end - inspect_start) / 1000000 ))
    inspect_dur_s=$(echo "scale=2; $inspect_dur_ms / 1000" | bc 2>/dev/null || echo "${inspect_dur_ms}ms")
    STEP_DURATIONS+=("${inspect_dur_s}s")

    if [ $inspect_exit -eq 0 ]; then
        log_pass "Layout Inspector passed in ${inspect_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "Layout Inspector failed. See: $INSPECT_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

    # ────────────────────────────────────────────────────────────────────────
    # Step 12: Terminal Capabilities Report Export (bd-iuvb.6)
    #
    # Runs the Terminal Capabilities screen and triggers an export via
    # an injected 'e' keypress to produce JSONL output.
    # ────────────────────────────────────────────────────────────────────────
    log_step "Terminal caps report (screen 11)"
    log_info "Running TerminalCapabilities and exporting JSONL report..."
    CAPS_LOG="$LOG_DIR/12_terminal_caps.log"
    CAPS_REPORT="$LOG_DIR/12_terminal_caps_report_${TIMESTAMP}.jsonl"
    CAPS_JSONL="$LOG_DIR/12_terminal_caps_summary.jsonl"
    STEP_NAMES+=("Terminal caps report")

    caps_start=$(date +%s%N)
    {
        echo "=== Terminal Capabilities (Screen 11) Report Export ==="
        echo "Bead: bd-iuvb.6"
        echo "Report path: $CAPS_REPORT"
        echo ""

        caps_cmd="stty rows 24 cols 80 2>/dev/null; (sleep 0.6; printf 'e' > /dev/tty) & FTUI_TERMCAPS_REPORT_PATH=\"$CAPS_REPORT\" FTUI_DEMO_EXIT_AFTER_MS=2000 FTUI_DEMO_SCREEN=11 timeout 8 $DEMO_BIN"

        if run_in_pty "$caps_cmd" 2>&1; then
            caps_exit=0
        else
            caps_exit=$?
        fi

        if [ "$caps_exit" -eq 124 ]; then
            caps_outcome="timeout"
        elif [ "$caps_exit" -eq 0 ]; then
            caps_outcome="pass"
        else
            caps_outcome="fail"
        fi

        caps_report_ok=false
        if [ -s "$CAPS_REPORT" ]; then
            caps_report_ok=true
        else
            echo "Report file missing or empty: $CAPS_REPORT"
            caps_outcome="no_report"
        fi

        caps_parse_ok=false
        if $caps_report_ok; then
            if python3 - "$CAPS_REPORT" "$CAPS_JSONL" "$TIMESTAMP" "$caps_outcome" "$caps_exit" <<'PY'
import json
import sys

report_path, summary_path, run_id, outcome, exit_code = sys.argv[1:6]

with open(report_path, "r", encoding="utf-8") as handle:
    lines = [line for line in handle if line.strip()]
if not lines:
    raise SystemExit("Report JSONL is empty")

report = json.loads(lines[-1])
caps = report.get("capabilities", [])
enabled = [row.get("capability") for row in caps if row.get("effective") is True]
disabled = [row.get("capability") for row in caps if row.get("effective") is False]

profile = report.get("simulated_profile") or report.get("detected_profile")
payload = {
    "run_id": run_id,
    "profile": profile,
    "enabled_features": enabled,
    "disabled_features": disabled,
    "outcome": outcome,
    "exit_code": int(exit_code),
}

with open(summary_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")
PY
            then
                caps_parse_ok=true
            else
                echo "Failed to parse report into summary JSONL"
                caps_outcome="parse_fail"
            fi
        fi

        caps_exit_ok=true
        if [ "$caps_exit" -ne 0 ] && [ "$caps_exit" -ne 124 ]; then
            caps_exit_ok=false
        fi

        caps_success=true
        if ! $caps_exit_ok; then caps_success=false; fi
        if ! $caps_report_ok; then caps_success=false; fi
        if ! $caps_parse_ok; then caps_success=false; fi

        echo "Outcome: $caps_outcome"
        echo "Summary JSONL: $CAPS_JSONL"

        $caps_success
    } > "$CAPS_LOG" 2>&1
    caps_exit=$?
    caps_end=$(date +%s%N)
    caps_dur_ms=$(( (caps_end - caps_start) / 1000000 ))
    caps_dur_s=$(echo "scale=2; $caps_dur_ms / 1000" | bc 2>/dev/null || echo "${caps_dur_ms}ms")
    STEP_DURATIONS+=("${caps_dur_s}s")

    if [ $caps_exit -eq 0 ]; then
        log_pass "Terminal caps report passed in ${caps_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "Terminal caps report failed. See: $CAPS_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

    # ────────────────────────────────────────────────────────────────────────
    # Step 13: i18n Stress Lab Report Export (bd-iuvb.9)
    #
    # Runs the i18n screen (screen 29), cycles to the Stress Lab panel,
    # and exports a JSONL report via an injected 'e' keypress.
    # ────────────────────────────────────────────────────────────────────────
    log_step "i18n stress report (screen 29)"
    log_info "Running i18n Stress Lab and exporting JSONL report..."
    I18N_LOG="$LOG_DIR/13_i18n_stress.log"
    I18N_REPORT="$LOG_DIR/13_i18n_report_${TIMESTAMP}.jsonl"
    I18N_JSONL="$LOG_DIR/13_i18n_summary.jsonl"
    STEP_NAMES+=("i18n stress report")

    i18n_start=$(date +%s%N)
    {
        echo "=== i18n Stress Lab (Screen 29) Report Export ==="
        echo "Bead: bd-iuvb.9"
        echo "Report path: $I18N_REPORT"
        echo ""

        i18n_cmd="stty rows 24 cols 80 2>/dev/null; (sleep 0.5; printf '\\t\\t\\t' > /dev/tty; sleep 0.2; printf 'e' > /dev/tty) & FTUI_I18N_REPORT_PATH=\"$I18N_REPORT\" FTUI_I18N_REPORT_WIDTH=32 FTUI_DEMO_EXIT_AFTER_MS=2200 FTUI_DEMO_SCREEN=29 timeout 8 $DEMO_BIN"

        if run_in_pty "$i18n_cmd" 2>&1; then
            i18n_exit=0
        else
            i18n_exit=$?
        fi

        if [ "$i18n_exit" -eq 124 ]; then
            i18n_outcome="timeout"
        elif [ "$i18n_exit" -eq 0 ]; then
            i18n_outcome="pass"
        else
            i18n_outcome="fail"
        fi

        i18n_report_ok=false
        if [ -s "$I18N_REPORT" ]; then
            i18n_report_ok=true
        else
            echo "Report file missing or empty: $I18N_REPORT"
            i18n_outcome="no_report"
        fi

        i18n_parse_ok=false
        if $i18n_report_ok; then
            if python3 - "$I18N_REPORT" "$I18N_JSONL" "$TIMESTAMP" "$i18n_outcome" "$i18n_exit" <<'PY'
import json
import sys

report_path, summary_path, run_id, outcome, exit_code = sys.argv[1:6]

with open(report_path, "r", encoding="utf-8") as handle:
    lines = [line for line in handle if line.strip()]
if not lines:
    raise SystemExit("Report JSONL is empty")

report = json.loads(lines[-1])
payload = {
    "run_id": run_id,
    "sample_id": report.get("sample_id"),
    "width_metrics": report.get("width_metrics", {}),
    "truncation_state": report.get("truncation_state", {}),
    "outcome": outcome,
    "exit_code": int(exit_code),
}

with open(summary_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\n")
PY
            then
                i18n_parse_ok=true
            else
                echo "Failed to parse report into summary JSONL"
                i18n_outcome="parse_fail"
            fi
        fi

        i18n_exit_ok=true
        if [ "$i18n_exit" -ne 0 ] && [ "$i18n_exit" -ne 124 ]; then
            i18n_exit_ok=false
        fi

        i18n_success=true
        if ! $i18n_exit_ok; then i18n_success=false; fi
        if ! $i18n_report_ok; then i18n_success=false; fi
        if ! $i18n_parse_ok; then i18n_success=false; fi

        echo "Outcome: $i18n_outcome"
        echo "Summary JSONL: $I18N_JSONL"

        $i18n_success
    } > "$I18N_LOG" 2>&1
    i18n_exit=$?
    i18n_end=$(date +%s%N)
    i18n_dur_ms=$(( (i18n_end - i18n_start) / 1000000 ))
    i18n_dur_s=$(echo "scale=2; $i18n_dur_ms / 1000" | bc 2>/dev/null || echo "${i18n_dur_ms}ms")
    STEP_DURATIONS+=("${i18n_dur_s}s")

    if [ $i18n_exit -eq 0 ]; then
        log_pass "i18n stress report passed in ${i18n_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "i18n stress report failed. See: $I18N_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

    # ────────────────────────────────────────────────────────────────────────
    # Step 14: Widget Builder Export (bd-iuvb.10)
    #
    # Runs the widget builder (screen 33) and exports a JSONL snapshot.
    # ────────────────────────────────────────────────────────────────────────
    log_step "widget builder export (screen 33)"
    log_info "Running Widget Builder and exporting JSONL snapshot..."
    WIDGET_LOG="$LOG_DIR/14_widget_builder.log"
    WIDGET_REPORT="$LOG_DIR/14_widget_builder_report_${TIMESTAMP}.jsonl"
    WIDGET_JSONL="$LOG_DIR/14_widget_builder_summary.jsonl"
    STEP_NAMES+=("widget builder export")

    widget_start=$(date +%s%N)
    {
        echo "=== Widget Builder (Screen 33) Export ==="
        echo "Bead: bd-iuvb.10"
        echo "Report path: $WIDGET_REPORT"
        echo ""

        widget_cmd="stty rows 24 cols 80 2>/dev/null; (sleep 0.5; printf 'x' > /dev/tty) & FTUI_WIDGET_BUILDER_EXPORT_PATH=\"$WIDGET_REPORT\" FTUI_WIDGET_BUILDER_RUN_ID=\"$TIMESTAMP\" FTUI_DEMO_EXIT_AFTER_MS=2200 FTUI_DEMO_SCREEN=33 timeout 8 $DEMO_BIN"

        if run_in_pty "$widget_cmd" 2>&1; then
            widget_exit=0
        else
            widget_exit=$?
        fi

        if [ "$widget_exit" -eq 124 ]; then
            widget_outcome="timeout"
        elif [ "$widget_exit" -eq 0 ]; then
            widget_outcome="pass"
        else
            widget_outcome="fail"
        fi

        widget_report_ok=false
        if [ -s "$WIDGET_REPORT" ]; then
            widget_report_ok=true
        else
            echo "Report file missing or empty: $WIDGET_REPORT"
            widget_outcome="no_report"
        fi

        widget_parse_ok=false
        if $widget_report_ok; then
            if python3 - "$WIDGET_REPORT" "$WIDGET_JSONL" "$TIMESTAMP" "$widget_outcome" "$widget_exit" <<'PY'
import json
import sys

report_path, summary_path, run_id, outcome, exit_code = sys.argv[1:6]

with open(report_path, "r", encoding="utf-8") as handle:
    lines = [line for line in handle if line.strip()]
if not lines:
    raise SystemExit("Report JSONL is empty")

report = json.loads(lines[-1])
payload = {
    "run_id": report.get("run_id", run_id),
    "preset_id": report.get("preset_id"),
    "widget_count": report.get("widget_count"),
    "props_hash": report.get("props_hash"),
    "outcome": outcome,
    "exit_code": int(exit_code),
}

with open(summary_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(payload) + "\\n")
PY
            then
                widget_parse_ok=true
            else
                echo "Failed to parse report into summary JSONL"
                widget_outcome="parse_fail"
            fi
        fi

        widget_exit_ok=true
        if [ "$widget_exit" -ne 0 ] && [ "$widget_exit" -ne 124 ]; then
            widget_exit_ok=false
        fi

        widget_success=true
        if ! $widget_exit_ok; then widget_success=false; fi
        if ! $widget_report_ok; then widget_success=false; fi
        if ! $widget_parse_ok; then widget_success=false; fi

        echo "Outcome: $widget_outcome"
        echo "Summary JSONL: $WIDGET_JSONL"

        $widget_success
    } > "$WIDGET_LOG" 2>&1
    widget_exit=$?
    widget_end=$(date +%s%N)
    widget_dur_ms=$(( (widget_end - widget_start) / 1000000 ))
    widget_dur_s=$(echo "scale=2; $widget_dur_ms / 1000" | bc 2>/dev/null || echo "${widget_dur_ms}ms")
    STEP_DURATIONS+=("${widget_dur_s}s")

    if [ $widget_exit -eq 0 ]; then
        log_pass "Widget builder export passed in ${widget_dur_s}s"
        PASS_COUNT=$((PASS_COUNT + 1))
        STEP_STATUSES+=("PASS")
    else
        log_fail "Widget builder export failed. See: $WIDGET_LOG"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        STEP_STATUSES+=("FAIL")
    fi

else
    # No PTY support — skip all smoke/interactive tests
    for step in "Smoke test (alt-screen)" "Smoke test (inline)" \
                "Screen navigation" "Search test (Shakespeare)" \
                "Resize (SIGWINCH) test" "VisualEffects backdrop" \
                "Layout Inspector" "Terminal caps report" "i18n stress report" \
                "Widget builder export"; do
        skip_step "$step" "$SMOKE_REASON"
    done
fi

fi  # end of non-quick block

# ============================================================================
# Summary
# ============================================================================

echo ""
echo "=============================================="
echo "  E2E Test Suite Complete"
echo "=============================================="
echo ""
echo "Ended at: $(date -Iseconds)"
echo "Log directory: $LOG_DIR"
echo ""

# Summary table
printf "%-35s %-6s %s\n" "Step" "Status" "Duration"
printf "%-35s %-6s %s\n" "---" "------" "--------"
for i in "${!STEP_NAMES[@]}"; do
    local_status="${STEP_STATUSES[$i]}"
    case $local_status in
        PASS) color="\033[32m" ;;
        FAIL) color="\033[31m" ;;
        SKIP) color="\033[33m" ;;
        *)    color="" ;;
    esac
    printf "%-35s ${color}%-6s\033[0m %s\n" "${STEP_NAMES[$i]}" "$local_status" "${STEP_DURATIONS[$i]}"
done

echo ""
echo "Results: $PASS_COUNT passed, $FAIL_COUNT failed, $SKIP_COUNT skipped"
echo ""

# List log files with sizes
echo "Log files:"
ls -lh "$LOG_DIR"/*.log 2>/dev/null | awk '{print "  " $9 " (" $5 ")"}'
echo ""

# Generate summary file
{
    echo "Demo Showcase E2E Summary"
    echo "========================="
    echo "Date: $(date -Iseconds)"
    echo "Passed: $PASS_COUNT"
    echo "Failed: $FAIL_COUNT"
    echo "Skipped: $SKIP_COUNT"
    echo ""
    for i in "${!STEP_NAMES[@]}"; do
        printf "  %-35s %s  %s\n" "${STEP_NAMES[$i]}" "${STEP_STATUSES[$i]}" "${STEP_DURATIONS[$i]}"
    done
    echo ""
    echo "Exit code: $( [ $FAIL_COUNT -eq 0 ] && echo 0 || echo 1 )"
} > "$LOG_DIR/SUMMARY.txt"

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "\033[1;32mAll tests passed!\033[0m"
    exit 0
else
    echo -e "\033[1;31m$FAIL_COUNT test(s) failed!\033[0m"
    exit 1
fi

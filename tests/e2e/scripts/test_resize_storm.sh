#!/bin/bash
set -euo pipefail

# E2E tests for Resize Storm scenarios (bd-1rz0.9)
#
# Hammers resize events at the demo showcase and verifies no flicker/tearing.
# Uses deterministic storm patterns with verbose JSONL logging.
#
# JSONL Schema:
# {"event":"env","run_id":"...","timestamp":"...","seed":N,"storm_pattern":"...","env":{...}}
# {"event":"storm_config","run_id":"...","pattern":"...","event_count":N,"initial_size":"WxH"}
# {"event":"resize","run_id":"...","seq":N,"from":"WxH","to":"WxH","delay_ms":N,"timestamp":"..."}
# {"event":"frame_capture","run_id":"...","seq":N,"width":N,"height":N,"checksum":"sha256:...","bytes":N}
# {"event":"artifact_check","run_id":"...","check":"...","result":"pass|fail","details":"..."}
# {"event":"flicker_analysis","run_id":"...","flicker_free":true,"jsonl":"path","exit_code":0}
# {"event":"complete","run_id":"...","outcome":"pass|fail","total_resizes":N,"total_ms":N,"checksums":[...]}
#
# Usage:
#   ./test_resize_storm.sh                    # Run all storm patterns
#   STORM_SEED=42 ./test_resize_storm.sh      # Deterministic mode
#   STORM_PATTERN=burst ./test_resize_storm.sh # Single pattern
#   STORM_COUNT=50 ./test_resize_storm.sh     # Override event count

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LIB_DIR="$SCRIPT_DIR/../lib"

# shellcheck source=/dev/null
source "$LIB_DIR/common.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/logging.sh"
# shellcheck source=/dev/null
source "$LIB_DIR/pty.sh"

# Configuration
STORM_SEED="${STORM_SEED:-0}"
STORM_PATTERN="${STORM_PATTERN:-all}"
STORM_COUNT="${STORM_COUNT:-20}"
STORM_LOG_DIR="${STORM_LOG_DIR:-$E2E_LOG_DIR/resize_storm}"
STORM_INTERVAL_MS="${STORM_INTERVAL_MS:-50}"

mkdir -p "$STORM_LOG_DIR"

# Master JSONL log
STORM_JSONL="$STORM_LOG_DIR/resize_storm_$(date +%Y%m%d_%H%M%S).jsonl"

# Artifact detection thresholds
MAX_GHOSTING_ROWS=0
MAX_TEAR_ARTIFACTS=0

# Log environment
log_storm_env() {
    local run_id="$1"
    local pattern="$2"
    cat >> "$STORM_JSONL" <<EOF
{"event":"env","run_id":"$run_id","timestamp":"$(date -Iseconds)","seed":$STORM_SEED,"storm_pattern":"$pattern","env":{"term":"${TERM:-}","colorterm":"${COLORTERM:-}","columns":"${COLUMNS:-}","lines":"${LINES:-}"}}
{"event":"git","run_id":"$run_id","commit":"$(git rev-parse HEAD 2>/dev/null || echo 'N/A')","branch":"$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo 'N/A')"}
EOF
}

# Log storm configuration
log_storm_config() {
    local run_id="$1"
    local pattern="$2"
    local event_count="$3"
    local initial_width="$4"
    local initial_height="$5"
    cat >> "$STORM_JSONL" <<EOF
{"event":"storm_config","run_id":"$run_id","pattern":"$pattern","event_count":$event_count,"initial_size":"${initial_width}x${initial_height}"}
EOF
}

# Log individual resize event
log_resize() {
    local run_id="$1"
    local seq="$2"
    local from_w="$3"
    local from_h="$4"
    local to_w="$5"
    local to_h="$6"
    local delay_ms="$7"
    cat >> "$STORM_JSONL" <<EOF
{"event":"resize","run_id":"$run_id","seq":$seq,"from":"${from_w}x${from_h}","to":"${to_w}x${to_h}","delay_ms":$delay_ms,"timestamp":"$(date -Iseconds)"}
EOF
}

# Log frame capture
log_frame_capture() {
    local run_id="$1"
    local seq="$2"
    local width="$3"
    local height="$4"
    local checksum="$5"
    local bytes="$6"
    cat >> "$STORM_JSONL" <<EOF
{"event":"frame_capture","run_id":"$run_id","seq":$seq,"width":$width,"height":$height,"checksum":"sha256:$checksum","bytes":$bytes}
EOF
}

# Log artifact check result
log_artifact_check() {
    local run_id="$1"
    local check="$2"
    local result="$3"
    local details="$4"
    cat >> "$STORM_JSONL" <<EOF
{"event":"artifact_check","run_id":"$run_id","check":"$check","result":"$result","details":"$details"}
EOF
}

# Log flicker analysis result
log_flicker_analysis() {
    local run_id="$1"
    local flicker_free="$2"
    local jsonl_path="$3"
    local exit_code="$4"
    cat >> "$STORM_JSONL" <<EOF
{"event":"flicker_analysis","run_id":"$run_id","flicker_free":$flicker_free,"jsonl":"$jsonl_path","exit_code":$exit_code}
EOF
}

# Log completion
log_storm_complete() {
    local run_id="$1"
    local outcome="$2"
    local total_resizes="$3"
    local total_ms="$4"
    local checksums="$5"
    cat >> "$STORM_JSONL" <<EOF
{"event":"complete","run_id":"$run_id","outcome":"$outcome","total_resizes":$total_resizes,"total_ms":$total_ms,"checksums":[$checksums]}
EOF
}

# Compute checksum of file
compute_checksum() {
    local file="$1"
    if [[ -f "$file" ]]; then
        if command -v sha256sum >/dev/null 2>&1; then
            sha256sum "$file" | cut -d' ' -f1 | head -c 16
        elif command -v md5sum >/dev/null 2>&1; then
            md5sum "$file" | cut -d' ' -f1 | head -c 16
        else
            local size
            size=$(wc -c < "$file")
            printf "%08x%08x" "$size" "$(head -c 64 "$file" | cksum | cut -d' ' -f1)"
        fi
    else
        echo "0000000000000000"
    fi
}

# Generate resize storm events based on pattern
# Returns: JSON array of resize events
generate_storm_events() {
    local pattern="$1"
    local count="$2"
    local seed="$3"
    local initial_w="${4:-80}"
    local initial_h="${5:-24}"

    # Use awk for deterministic pseudo-random generation
    awk -v pattern="$pattern" -v count="$count" -v seed="$seed" \
        -v init_w="$initial_w" -v init_h="$initial_h" '
    BEGIN {
        srand(seed)
        w = init_w
        h = init_h

        if (pattern == "burst") {
            # Random sizes within reasonable bounds
            for (i = 0; i < count; i++) {
                new_w = 40 + int(rand() * 160)  # 40-200
                new_h = 10 + int(rand() * 50)   # 10-60
                print w "," h "," new_w "," new_h
                w = new_w
                h = new_h
            }
        } else if (pattern == "oscillate") {
            # Alternate between two sizes
            w1 = 80; h1 = 24
            w2 = 120; h2 = 40
            for (i = 0; i < count; i++) {
                if (i % 2 == 0) {
                    print w "," h "," w2 "," h2
                    w = w2; h = h2
                } else {
                    print w "," h "," w1 "," h1
                    w = w1; h = h1
                }
            }
        } else if (pattern == "sweep") {
            # Gradual size increase
            start_w = 40; start_h = 12
            end_w = 200; end_h = 60
            w = start_w; h = start_h
            for (i = 0; i < count; i++) {
                new_w = start_w + int((end_w - start_w) * i / count)
                new_h = start_h + int((end_h - start_h) * i / count)
                print w "," h "," new_w "," new_h
                w = new_w; h = new_h
            }
        } else if (pattern == "shrink") {
            # Progressive shrinking (stress test for ghosting)
            for (i = 0; i < count; i++) {
                new_w = w - 2 - int(rand() * 5)
                new_h = h - 1 - int(rand() * 2)
                if (new_w < 20) new_w = 20 + int(rand() * 10)
                if (new_h < 8) new_h = 8 + int(rand() * 4)
                print w "," h "," new_w "," new_h
                w = new_w; h = new_h
            }
        } else if (pattern == "jitter") {
            # Small rapid changes (tests coalescing)
            for (i = 0; i < count; i++) {
                delta_w = int(rand() * 5) - 2  # -2 to +2
                delta_h = int(rand() * 3) - 1  # -1 to +1
                new_w = w + delta_w
                new_h = h + delta_h
                if (new_w < 40) new_w = 40
                if (new_w > 200) new_w = 200
                if (new_h < 10) new_h = 10
                if (new_h > 60) new_h = 60
                print w "," h "," new_w "," new_h
                w = new_w; h = new_h
            }
        } else {
            # Default: mixed pattern
            for (i = 0; i < count; i++) {
                new_w = 40 + int(rand() * 160)
                new_h = 10 + int(rand() * 50)
                print w "," h "," new_w "," new_h
                w = new_w; h = new_h
            }
        }
    }
    '
}

# Check for ghosting artifacts (stale content after shrink)
check_ghosting() {
    local output_file="$1"
    local expected_height="$2"

    # Look for content below expected height boundary
    # This is a heuristic - ghosting shows as non-empty lines past the expected region
    local ghost_count=0

    if [[ -f "$output_file" ]] && command -v strings >/dev/null 2>&1; then
        # Count significant lines in output
        local line_count
        line_count=$(strings "$output_file" | wc -l)
        if [[ "$line_count" -gt $((expected_height + 5)) ]]; then
            ghost_count=$((line_count - expected_height - 5))
        fi
    fi

    echo "$ghost_count"
}

# Check for tearing artifacts (incomplete escape sequences)
check_tearing() {
    local output_file="$1"
    local tear_count=0

    if [[ -f "$output_file" ]]; then
        # Look for incomplete CSI sequences (ESC [ without terminator)
        # This indicates a partial write that got interrupted
        if command -v grep >/dev/null 2>&1; then
            # Count ESC sequences that don't have a proper terminator nearby
            local esc_count
            esc_count=$(grep -aoP '\x1b\[' "$output_file" 2>/dev/null | wc -l || echo 0)
            local complete_count
            complete_count=$(grep -aoP '\x1b\[[0-9;]*[A-Za-z]' "$output_file" 2>/dev/null | wc -l || echo 0)
            if [[ "$esc_count" -gt "$complete_count" ]]; then
                tear_count=$((esc_count - complete_count))
            fi
        fi
    fi

    echo "$tear_count"
}

# Check if harness binary exists
if [[ ! -x "${E2E_HARNESS_BIN:-}" ]]; then
    LOG_FILE="$STORM_LOG_DIR/storm_missing.log"
    for pattern in burst oscillate sweep shrink jitter mixed; do
        log_test_skip "storm_$pattern" "ftui-harness binary missing"
        record_result "storm_$pattern" "skipped" 0 "$LOG_FILE" "binary missing"
    done
    exit 0
fi

# Run a resize storm scenario
run_storm_scenario() {
    local pattern="$1"
    local count="${2:-$STORM_COUNT}"
    local interval_ms="${3:-$STORM_INTERVAL_MS}"
    local initial_w="${4:-80}"
    local initial_h="${5:-24}"

    local run_id
    run_id="storm_$(date +%s%N | head -c 16)"
    LOG_FILE="$STORM_LOG_DIR/storm_${pattern}.log"
    local output_file="$STORM_LOG_DIR/storm_${pattern}.pty"
    local resize_schedule="$STORM_LOG_DIR/storm_${pattern}_schedule.txt"

    log_test_start "storm_$pattern"
    log_storm_env "$run_id" "$pattern"
    log_storm_config "$run_id" "$pattern" "$count" "$initial_w" "$initial_h"

    local start_ms
    start_ms="$(date +%s%3N)"

    # Generate storm schedule
    generate_storm_events "$pattern" "$count" "$STORM_SEED" "$initial_w" "$initial_h" > "$resize_schedule"

    # Calculate total timeout: base time + (count * interval + buffer)
    local total_timeout_s
    total_timeout_s=$(( (count * interval_ms / 1000) + 5 ))
    if [[ "$total_timeout_s" -lt 3 ]]; then
        total_timeout_s=3
    fi

    # Build resize event list for Python script
    local resize_events=""
    local seq=0
    local prev_w="$initial_w"
    local prev_h="$initial_h"
    local checksums=""

    while IFS=',' read -r from_w from_h to_w to_h; do
        if [[ -n "$to_w" && -n "$to_h" ]]; then
            log_resize "$run_id" "$seq" "$from_w" "$from_h" "$to_w" "$to_h" "$interval_ms"
            resize_events+="${to_w},${to_h},${interval_ms};"
            prev_w="$to_w"
            prev_h="$to_h"
            seq=$((seq + 1))
        fi
    done < "$resize_schedule"

    # Run harness with multi-resize support
    local pty_env=(
        PTY_COLS="$initial_w"
        PTY_ROWS="$initial_h"
        FTUI_HARNESS_EXIT_AFTER_MS="$((total_timeout_s * 1000))"
        FTUI_HARNESS_LOG_LINES=5
        FTUI_HARNESS_SUPPRESS_WELCOME=1
        PTY_TIMEOUT="$((total_timeout_s + 2))"
        PTY_CANONICALIZE=1
        PTY_TEST_NAME="storm_$pattern"
        PTY_JSONL="$STORM_LOG_DIR/storm_pty.jsonl"
        PTY_RESIZE_SCHEDULE="$resize_events"
        PTY_RESIZE_INTERVAL_MS="$interval_ms"
    )

    # Use extended PTY runner with storm support
    if ! env "${pty_env[@]}" run_storm_pty "$output_file" "$E2E_HARNESS_BIN"; then
        log_error "PTY execution failed for storm_$pattern"
    fi

    local end_ms
    end_ms="$(date +%s%3N)"
    local duration_ms=$((end_ms - start_ms))

    # Compute final checksum
    local final_checksum
    final_checksum="$(compute_checksum "$output_file")"
    local file_size=0
    if [[ -f "$output_file" ]]; then
        file_size=$(wc -c < "$output_file" | tr -d ' ')
    fi
    log_frame_capture "$run_id" "$seq" "$prev_w" "$prev_h" "$final_checksum" "$file_size"
    checksums="\"sha256:$final_checksum\""

    # Artifact checks
    local outcome="pass"

    # Check for ghosting
    local ghost_count
    ghost_count=$(check_ghosting "$output_file" "$prev_h")
    if [[ "$ghost_count" -gt "$MAX_GHOSTING_ROWS" ]]; then
        log_artifact_check "$run_id" "ghosting" "fail" "found $ghost_count ghost rows"
        outcome="fail"
    else
        log_artifact_check "$run_id" "ghosting" "pass" "no ghost rows detected"
    fi

    # Check for tearing
    local tear_count
    tear_count=$(check_tearing "$output_file")
    if [[ "$tear_count" -gt "$MAX_TEAR_ARTIFACTS" ]]; then
        log_artifact_check "$run_id" "tearing" "fail" "found $tear_count incomplete sequences"
        outcome="fail"
    else
        log_artifact_check "$run_id" "tearing" "pass" "no incomplete sequences"
    fi

    # Flicker detection analysis (uses harness analyzer)
    local flicker_jsonl="$STORM_LOG_DIR/storm_${pattern}_flicker.jsonl"
    local flicker_exit=0
    if ! FTUI_HARNESS_FLICKER_ANALYZE=1 \
        FTUI_HARNESS_FLICKER_INPUT="$output_file" \
        FTUI_HARNESS_FLICKER_RUN_ID="$run_id" \
        FTUI_HARNESS_FLICKER_JSONL="$flicker_jsonl" \
        "$E2E_HARNESS_BIN" >/dev/null 2>&1; then
        flicker_exit=$?
    fi

    local flicker_free="null"
    if [[ -f "$flicker_jsonl" ]]; then
        if command -v jq >/dev/null 2>&1; then
            flicker_free=$(jq -r 'select(.event_type=="analysis_complete") | .details.stats.flicker_free' "$flicker_jsonl" | tail -n1)
        else
            flicker_free=$(grep -a '"event_type":"analysis_complete"' "$flicker_jsonl" | tail -n1 | sed -E 's/.*"flicker_free":(true|false).*/\\1/')
        fi
    fi

    log_flicker_analysis "$run_id" "${flicker_free:-null}" "$flicker_jsonl" "$flicker_exit"
    if [[ "$flicker_free" != "true" ]]; then
        outcome="fail"
    fi

    log_storm_complete "$run_id" "$outcome" "$seq" "$duration_ms" "$checksums"

    if [[ "$outcome" == "pass" ]]; then
        log_test_pass "storm_$pattern"
        record_result "storm_$pattern" "passed" "$duration_ms" "$LOG_FILE"
        return 0
    else
        log_test_fail "storm_$pattern" "artifacts detected"
        record_result "storm_$pattern" "failed" "$duration_ms" "$LOG_FILE" "artifacts detected"
        return 1
    fi
}

# Extended PTY runner with multi-resize storm support
run_storm_pty() {
    local output_file="$1"
    shift

    if [[ -z "${E2E_PYTHON:-}" ]]; then
        echo "E2E_PYTHON is not set" >&2
        return 1
    fi

    "$E2E_PYTHON" - "$@" <<'STORM_PY'
import codecs
import os
import pty
import select
import subprocess
import sys
import time
import signal

cmd = sys.argv[1:]
if not cmd:
    print("No command provided", file=sys.stderr)
    sys.exit(2)

output_path = os.environ.get("PTY_OUTPUT")
if not output_path:
    print("PTY_OUTPUT not set", file=sys.stderr)
    sys.exit(2)

timeout = float(os.environ.get("PTY_TIMEOUT", "10"))
cols = int(os.environ.get("PTY_COLS", "80"))
rows = int(os.environ.get("PTY_ROWS", "24"))
drain_timeout = float(os.environ.get("PTY_DRAIN_TIMEOUT_MS", "200")) / 1000.0
read_poll = float(os.environ.get("PTY_READ_POLL_MS", "50")) / 1000.0
read_chunk = int(os.environ.get("PTY_READ_CHUNK", "4096"))
resize_schedule_str = os.environ.get("PTY_RESIZE_SCHEDULE", "")
base_interval_ms = int(os.environ.get("PTY_RESIZE_INTERVAL_MS", "50"))

# Parse resize schedule: "cols,rows,delay_ms;cols,rows,delay_ms;..."
resize_events = []
if resize_schedule_str:
    for event_str in resize_schedule_str.strip(';').split(';'):
        if event_str:
            parts = event_str.split(',')
            if len(parts) >= 2:
                new_cols = int(parts[0])
                new_rows = int(parts[1])
                delay_ms = int(parts[2]) if len(parts) > 2 else base_interval_ms
                resize_events.append((new_cols, new_rows, delay_ms))

master_fd, slave_fd = pty.openpty()

try:
    import fcntl
    import struct
    import termios
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    fcntl.ioctl(slave_fd, termios.TIOCSWINSZ, winsize)
except Exception:
    pass

start = time.monotonic()
deadline = start + timeout

proc = subprocess.Popen(
    cmd,
    stdin=slave_fd,
    stdout=slave_fd,
    stderr=slave_fd,
    close_fds=True,
    env=os.environ.copy(),
    start_new_session=True,
)
slave_fd_open = slave_fd

captured = bytearray()
last_data = start
terminate_at = None
stop_at = None

# Schedule resize events
resize_idx = 0
next_resize_at = start + 0.3  # Start resizes after 300ms warmup
resize_interval = base_interval_ms / 1000.0

try:
    while True:
        now = time.monotonic()

        # Handle scheduled resize events
        if resize_idx < len(resize_events) and now >= next_resize_at:
            new_cols, new_rows, delay_ms = resize_events[resize_idx]
            try:
                import fcntl
                import struct
                import termios
                winsize = struct.pack("HHHH", new_rows, new_cols, 0, 0)
                try:
                    fcntl.ioctl(master_fd, termios.TIOCSWINSZ, winsize)
                except Exception:
                    pass
                if slave_fd_open is not None:
                    try:
                        fcntl.ioctl(slave_fd_open, termios.TIOCSWINSZ, winsize)
                    except Exception:
                        pass
                try:
                    os.killpg(proc.pid, signal.SIGWINCH)
                except Exception:
                    pass
            except Exception as e:
                print(f"Resize failed: {e}", file=sys.stderr)

            resize_idx += 1
            next_resize_at = now + (delay_ms / 1000.0)

        # Handle timeout
        if terminate_at is None and now >= deadline:
            terminate_at = now + 0.3
            stop_at = terminate_at + drain_timeout
            try:
                os.killpg(proc.pid, signal.SIGTERM)
            except Exception:
                try:
                    proc.terminate()
                except Exception:
                    pass

        if terminate_at is not None and now >= terminate_at:
            if proc.poll() is None:
                try:
                    os.killpg(proc.pid, signal.SIGKILL)
                except Exception:
                    try:
                        proc.kill()
                    except Exception:
                        pass

        # Read output
        rlist, _, _ = select.select([master_fd], [], [], read_poll)
        if rlist:
            try:
                chunk = os.read(master_fd, read_chunk)
            except OSError:
                break
            if not chunk:
                break
            captured.extend(chunk)
            last_data = now

        exit_code = proc.poll()
        if exit_code is not None:
            if now - last_data >= drain_timeout:
                break

        if stop_at is not None and now >= stop_at and (now - last_data >= drain_timeout):
            break
finally:
    try:
        os.close(master_fd)
    except Exception:
        pass
    if slave_fd_open is not None:
        try:
            os.close(slave_fd_open)
        except Exception:
            pass

exit_code = proc.poll()
if exit_code is None:
    exit_code = 124

with open(output_path, "wb") as handle:
    handle.write(captured)

sys.exit(exit_code)
STORM_PY
}

# Log summary header
log_info "=========================================="
log_info "Resize Storm E2E Tests (bd-1rz0.9)"
log_info "=========================================="
log_info "STORM_SEED: $STORM_SEED"
log_info "STORM_PATTERN: $STORM_PATTERN"
log_info "STORM_COUNT: $STORM_COUNT"
log_info "STORM_INTERVAL_MS: $STORM_INTERVAL_MS"
log_info "Log directory: $STORM_LOG_DIR"
log_info "JSONL log: $STORM_JSONL"
log_info ""

FAILURES=0

if [[ "$STORM_PATTERN" == "all" ]]; then
    # Run all storm patterns
    run_storm_scenario "burst" "$STORM_COUNT" "$STORM_INTERVAL_MS" 80 24 || FAILURES=$((FAILURES + 1))
    run_storm_scenario "oscillate" "$STORM_COUNT" "$STORM_INTERVAL_MS" 80 24 || FAILURES=$((FAILURES + 1))
    run_storm_scenario "sweep" "$STORM_COUNT" "$STORM_INTERVAL_MS" 40 12 || FAILURES=$((FAILURES + 1))
    run_storm_scenario "shrink" "$STORM_COUNT" "$STORM_INTERVAL_MS" 120 40 || FAILURES=$((FAILURES + 1))
    run_storm_scenario "jitter" "$STORM_COUNT" "$STORM_INTERVAL_MS" 80 24 || FAILURES=$((FAILURES + 1))
    run_storm_scenario "mixed" "$STORM_COUNT" "$STORM_INTERVAL_MS" 80 24 || FAILURES=$((FAILURES + 1))
else
    # Run single pattern
    run_storm_scenario "$STORM_PATTERN" "$STORM_COUNT" "$STORM_INTERVAL_MS" 80 24 || FAILURES=$((FAILURES + 1))
fi

# Summary
log_info ""
log_info "=========================================="
log_info "Resize Storm E2E Tests Complete"
log_info "=========================================="
log_info "Failures: $FAILURES"
log_info "JSONL log: $STORM_JSONL"

# Print reproduction command on failure
if [[ "$FAILURES" -gt 0 ]]; then
    log_error ""
    log_error "Reproduction command:"
    log_error "  STORM_SEED=$STORM_SEED STORM_PATTERN=$STORM_PATTERN ./tests/e2e/scripts/test_resize_storm.sh"
    log_error ""
    log_error "Artifacts directory: $STORM_LOG_DIR"
fi

exit "$FAILURES"

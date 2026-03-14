#!/usr/bin/env bash
# Pane profiling runner (bd-1y0ph)
#
# Captures repeatable benchmark output for the pane core, terminal adapter,
# and web pointer-capture paths into one artifact directory.
#
# Usage:
#   ./scripts/pane_profile.sh
#   ./scripts/pane_profile.sh --test
#   ./scripts/pane_profile.sh --perf-stat
#   ./scripts/pane_profile.sh --out-dir target/pane-profiling/custom

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUT_DIR="${PROJECT_ROOT}/target/pane-profiling/bd-1y0ph"
TEST_MODE=false
RESOURCE_STATS=false
PERF_STAT=false

if command -v rch >/dev/null 2>&1; then
    CARGO_RUNNER=(rch exec -- cargo)
else
    CARGO_RUNNER=(cargo)
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        --test)
            TEST_MODE=true
            shift
            ;;
        --out-dir)
            OUT_DIR="$2"
            shift 2
            ;;
        --time)
            RESOURCE_STATS=true
            shift
            ;;
        --perf-stat)
            PERF_STAT=true
            shift
            ;;
        -h|--help)
            cat <<EOF
Usage: $0 [--test] [--time] [--perf-stat] [--out-dir PATH]

  --test          Run Criterion targets in fast test mode.
  --time          Capture /usr/bin/time -v resource stats per bench.
  --perf-stat     Capture perf stat counters for representative pane benches.
  --out-dir PATH  Write captured outputs under PATH.
EOF
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

mkdir -p "$OUT_DIR"

bench_args=()
if [[ "$TEST_MODE" == "true" ]]; then
    bench_args+=(--test)
fi

run_bench() {
    local label="$1"
    shift
    local output_file="${OUT_DIR}/${label}.txt"
    local time_file="${OUT_DIR}/${label}.time.txt"
    echo "==> ${label}"
    if [[ "$RESOURCE_STATS" == "true" ]]; then
        if [[ ! -x /usr/bin/time ]]; then
            echo "ERROR: /usr/bin/time is required for --time mode" >&2
            exit 1
        fi
        /usr/bin/time -v -o "$time_file" "${CARGO_RUNNER[@]}" "$@" 2>&1 | tee "$output_file"
    else
        "${CARGO_RUNNER[@]}" "$@" 2>&1 | tee "$output_file"
    fi
}

find_bench_binary() {
    local prefix="$1"
    local binary

    binary="$(
        find "${PROJECT_ROOT}/target/release/deps" \
            -maxdepth 1 \
            -type f \
            -name "${prefix}-*" \
            ! -name '*.d' \
            -perm -u+x \
            -printf '%T@ %p\n' \
            | sort -n \
            | tail -n1 \
            | cut -d' ' -f2-
    )"

    if [[ -z "$binary" ]]; then
        echo "ERROR: no bench binary found for ${prefix}" >&2
        echo "Hint: run the matching cargo bench target first." >&2
        exit 1
    fi

    printf '%s\n' "$binary"
}

run_perf_stat() {
    local label="$1"
    local binary_prefix="$2"
    local benchmark_name="$3"
    local output_file="${OUT_DIR}/${label}.perfstat.txt"
    local binary

    if ! command -v perf >/dev/null 2>&1; then
        echo "ERROR: perf is required for --perf-stat mode" >&2
        exit 1
    fi

    binary="$(find_bench_binary "$binary_prefix")"
    echo "==> ${label} (perf stat)"
    perf stat -d -r 3 -o "$output_file" -- \
        "$binary" \
        "$benchmark_name" \
        --exact \
        --profile-time 2 \
        --noplot
}

run_bench \
    layout_bench \
    bench -p ftui-layout --bench layout_bench -- pane/core/ "${bench_args[@]}"

run_bench \
    pane_terminal_bench \
    bench -p ftui-runtime --bench pane_terminal_bench -- pane/terminal/ "${bench_args[@]}"

run_bench \
    pane_pointer_bench \
    bench -p ftui-web --bench pane_pointer_bench -- pane/web_pointer/ "${bench_args[@]}"

if [[ "$PERF_STAT" == "true" ]]; then
    run_perf_stat \
        layout_bench \
        layout_bench \
        "pane/core/timeline/apply_and_replay_32_ops"

    run_perf_stat \
        pane_terminal_bench \
        pane_terminal_bench \
        "pane/terminal/lifecycle/down_drag_120_up"

    run_perf_stat \
        pane_pointer_bench \
        pane_pointer_bench \
        "pane/web_pointer/lifecycle/down_ack_move_120_up"
fi

cat > "${OUT_DIR}/README.txt" <<EOF
Pane profiling artifacts for bd-1y0ph.

Files:
- layout_bench.txt          pane/core/* Criterion output
- pane_terminal_bench.txt   pane/terminal/* Criterion output
- pane_pointer_bench.txt    pane/web_pointer/* Criterion output
- *.time.txt                optional /usr/bin/time -v resource summaries
- *.perfstat.txt            optional perf stat counter summaries

Runner:
- ${0##*/}

Mode:
- TEST_MODE=${TEST_MODE}
- RESOURCE_STATS=${RESOURCE_STATS}
- PERF_STAT=${PERF_STAT}
EOF

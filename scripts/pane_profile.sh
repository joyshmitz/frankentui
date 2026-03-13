#!/usr/bin/env bash
# Pane profiling runner (bd-1y0ph)
#
# Captures repeatable benchmark output for the pane core, terminal adapter,
# and web pointer-capture paths into one artifact directory.
#
# Usage:
#   ./scripts/pane_profile.sh
#   ./scripts/pane_profile.sh --test
#   ./scripts/pane_profile.sh --out-dir target/pane-profiling/custom

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUT_DIR="${PROJECT_ROOT}/target/pane-profiling/bd-1y0ph"
TEST_MODE=false

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
        -h|--help)
            cat <<EOF
Usage: $0 [--test] [--out-dir PATH]

  --test          Run Criterion targets in fast test mode.
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
    echo "==> ${label}"
    "${CARGO_RUNNER[@]}" "$@" 2>&1 | tee "$output_file"
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

cat > "${OUT_DIR}/README.txt" <<EOF
Pane profiling artifacts for bd-1y0ph.

Files:
- layout_bench.txt          pane/core/* Criterion output
- pane_terminal_bench.txt   pane/terminal/* Criterion output
- pane_pointer_bench.txt    pane/web_pointer/* Criterion output

Runner:
- ${0##*/}

Mode:
- TEST_MODE=${TEST_MODE}
EOF

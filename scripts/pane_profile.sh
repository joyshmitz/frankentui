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
declare -A BENCH_BINARY_PATHS=()
declare -A BENCH_EXECUTED_PATHS=()
declare -A BENCH_WORKERS=()
declare -A BENCH_EXACT_BINARY_ARTIFACTS=()
declare -A BENCH_FETCH_ERRORS=()
EXACT_BINARY_DIR=""

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
EXACT_BINARY_DIR="${OUT_DIR}/executed-binaries"
mkdir -p "$EXACT_BINARY_DIR"

bench_args=()
if [[ "$TEST_MODE" == "true" ]]; then
    bench_args+=(--test)
fi

strip_ansi_file() {
    local input_file="$1"
    perl -pe 's/\e\[[0-9;]*[[:alpha:]]//g' "$input_file"
}

parse_bench_binary_from_output() {
    local output_file="$1"
    strip_ansi_file "$output_file" \
        | sed -nE 's#^[[:space:]]*Running benches/[^ ]+[[:space:]]+\(([^)]+)\)$#\1#p' \
        | tail -n1
}

parse_worker_from_output() {
    local output_file="$1"
    strip_ansi_file "$output_file" \
        | sed -nE 's#.*Selected worker: .* at ([^ ]+) .*#\1#p' \
        | tail -n1
}

fetch_exact_bench_binary() {
    local label="$1"
    local executed_path="$2"
    local worker="$3"
    local artifact_path="${EXACT_BINARY_DIR}/${label}-$(basename "$executed_path")"

    if [[ -z "$worker" ]]; then
        BENCH_FETCH_ERRORS["$label"]="worker_unknown"
        return 1
    fi

    if scp -O -q "$worker:$executed_path" "$artifact_path"; then
        chmod u+x "$artifact_path" || true
        BENCH_EXACT_BINARY_ARTIFACTS["$label"]="$artifact_path"
        BENCH_BINARY_PATHS["$label"]="$artifact_path"
        return 0
    fi

    BENCH_FETCH_ERRORS["$label"]="scp_failed"
    return 1
}

record_executed_binary_metadata() {
    local label="$1"
    local output_file="$2"
    local bench_binary
    local worker

    bench_binary="$(parse_bench_binary_from_output "$output_file")"
    worker="$(parse_worker_from_output "$output_file")"

    if [[ -n "$worker" ]]; then
        BENCH_WORKERS["$label"]="$worker"
    fi

    if [[ -z "$bench_binary" ]]; then
        BENCH_FETCH_ERRORS["$label"]="bench_binary_not_reported"
        return 0
    fi

    if [[ "$bench_binary" != /* ]]; then
        bench_binary="${PROJECT_ROOT}/${bench_binary}"
    fi

    BENCH_EXECUTED_PATHS["$label"]="$bench_binary"

    if [[ -x "$bench_binary" ]]; then
        BENCH_BINARY_PATHS["$label"]="$bench_binary"
        BENCH_EXACT_BINARY_ARTIFACTS["$label"]="$bench_binary"
        return 0
    fi

    fetch_exact_bench_binary "$label" "$bench_binary" "$worker" || true
}

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

    record_executed_binary_metadata "$label" "$output_file"
}

find_latest_local_bench_binary() {
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

    printf '%s\n' "$binary"
}

find_bench_binary() {
    local prefix="$1"
    local binary="${BENCH_BINARY_PATHS[$prefix]:-}"

    if [[ -n "$binary" && -x "$binary" ]]; then
        printf '%s\n' "$binary"
        return 0
    fi

    binary="$(find_latest_local_bench_binary "$prefix")"
    if [[ -n "$binary" ]]; then
        printf '%s\n' "$binary"
        return 0
    fi

    echo "ERROR: no bench binary found for ${prefix}" >&2
    echo "Hint: run the matching cargo bench target first." >&2
    exit 1
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

record_symbol_metadata() {
    local output_file="${OUT_DIR}/symbol_metadata.txt"
    local prefixes=(
        pane_profile_harness
        layout_bench
        pane_terminal_bench
        pane_pointer_bench
    )

    : > "$output_file"
    {
        echo "Pane bench symbol metadata"
        echo "Generated by: ${0##*/}"
        echo
    } >> "$output_file"

    for prefix in "${prefixes[@]}"; do
        local binary="${BENCH_EXACT_BINARY_ARTIFACTS[$prefix]:-}"
        local executed_path="${BENCH_EXECUTED_PATHS[$prefix]:-}"
        local worker="${BENCH_WORKERS[$prefix]:-local}"
        local fetch_error="${BENCH_FETCH_ERRORS[$prefix]:-}"
        local local_candidate=""
        local source="executed_local"
        local exact_status="available"

        if [[ -n "$binary" && "$binary" != "${executed_path}" ]]; then
            source="executed_remote_fetched"
        elif [[ -z "$binary" ]]; then
            source="executed_remote_missing"
            exact_status="missing"
            local_candidate="$(find_latest_local_bench_binary "$prefix")"
        fi

        {
            echo "== ${prefix} =="
            echo "executed_path=${executed_path:-unknown}"
            echo "worker=${worker}"
            echo "binary_source=${source}"
            echo "exact_binary_status=${exact_status}"
            if [[ -n "$binary" ]]; then
                echo "exact_binary_local=${binary}"
            else
                echo "exact_binary_local=missing"
            fi
            if [[ -n "$fetch_error" ]]; then
                echo "fetch_error=${fetch_error}"
            else
                echo "fetch_error=none"
            fi
            if [[ -n "$local_candidate" ]]; then
                echo "local_candidate=${local_candidate}"
            else
                echo "local_candidate=missing"
            fi
            if [[ -e "$binary" ]]; then
                file "$binary"
                if command -v readelf >/dev/null 2>&1; then
                    if readelf -S "$binary" | grep -q '\.debug_info'; then
                        echo "debug_info=present"
                    else
                        echo "debug_info=missing"
                    fi
                else
                    echo "debug_info=unknown (readelf unavailable)"
                fi
            else
                echo "local_binary_status=missing"
                echo "debug_info=unknown (exact executed binary not present locally)"
            fi
            echo
        } >> "$output_file"
    done
}

validate_symbol_metadata() {
    local output_file="${OUT_DIR}/symbol_metadata.txt"
    local prefixes=(
        pane_profile_harness
        layout_bench
        pane_terminal_bench
        pane_pointer_bench
    )

    for prefix in "${prefixes[@]}"; do
        if ! grep -A8 -F "== ${prefix} ==" "$output_file" | grep -q '^executed_path='; then
            echo "ERROR: symbol metadata missing executed_path for ${prefix}" >&2
            exit 1
        fi
        if ! grep -A8 -F "== ${prefix} ==" "$output_file" | grep -q '^binary_source='; then
            echo "ERROR: symbol metadata missing binary_source for ${prefix}" >&2
            exit 1
        fi
        if ! grep -A8 -F "== ${prefix} ==" "$output_file" | grep -q '^exact_binary_status='; then
            echo "ERROR: symbol metadata missing exact_binary_status for ${prefix}" >&2
            exit 1
        fi
    done
}

run_core_harness() {
    local output_file="${OUT_DIR}/pane_core_profile_harness.txt"
    local harness_dir="${OUT_DIR}/pane_core_profile_harness"
    local harness_args=(
        bench -p ftui-layout --bench pane_profile_harness --
        --out-dir "${harness_dir}"
    )

    if [[ "$TEST_MODE" == "true" ]]; then
        harness_args+=(--iterations 64 --warmup-iterations 8)
    else
        harness_args+=(--iterations 2000 --warmup-iterations 200)
    fi

    echo "==> pane_core_profile_harness"
    "${CARGO_RUNNER[@]}" "${harness_args[@]}" 2>&1 | tee "$output_file"

    record_executed_binary_metadata "pane_profile_harness" "$output_file"

    mkdir -p "$harness_dir"
    python3 - "$output_file" "$harness_dir" <<'PY'
import json
import pathlib
import sys

output_path = pathlib.Path(sys.argv[1])
harness_dir = pathlib.Path(sys.argv[2])
prefix_map = {
    "HARNESS_MANIFEST_JSON=": "manifest.json",
    "HARNESS_BASELINE_SNAPSHOT_JSON=": "baseline_snapshot.json",
    "HARNESS_FINAL_SNAPSHOT_JSON=": "final_snapshot.json",
    "HARNESS_RUN_LOG_JSON=": "run.log",
}

lines = output_path.read_text().splitlines()
payloads = {}
for line in lines:
    for prefix, filename in prefix_map.items():
        if line.startswith(prefix):
            payloads[filename] = json.loads(line[len(prefix):])
            break

missing = [filename for filename in prefix_map.values() if filename not in payloads]
if missing:
    raise SystemExit(f"missing harness payloads: {', '.join(missing)}")

for filename, payload in payloads.items():
    path = harness_dir / filename
    if filename == "run.log":
        path.write_text("".join(f"{entry}\n" for entry in payload))
    else:
        path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
PY
}

run_core_harness

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

record_symbol_metadata
validate_symbol_metadata

cat > "${OUT_DIR}/README.txt" <<EOF
Pane profiling artifacts for bd-1y0ph.

Files:
- pane_core_profile_harness.txt  long-lived pane-core harness output
- pane_core_profile_harness/     manifest, snapshots, and verbose log
- layout_bench.txt          pane/core/* Criterion output
- pane_terminal_bench.txt   pane/terminal/* Criterion output
- pane_pointer_bench.txt    pane/web_pointer/* Criterion output
- *.time.txt                optional /usr/bin/time -v resource summaries
- *.perfstat.txt            optional perf stat counter summaries
- executed-binaries/        fetched exact remote bench binaries when materialized locally
- symbol_metadata.txt       executed-path provenance + exact-binary trust/readiness

Runner:
- ${0##*/}

Mode:
- TEST_MODE=${TEST_MODE}
- RESOURCE_STATS=${RESOURCE_STATS}
- PERF_STAT=${PERF_STAT}
EOF

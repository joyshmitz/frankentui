#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TIMESTAMP_UTC="$(date -u +%Y%m%dT%H%M%SZ)"
RUN_ROOT="${1:-/tmp/ftui_runtime_shadow_compare/${TIMESTAMP_UTC}}"
SCENARIO="${2:-all}"
LOG_DIR="${RUN_ROOT}/logs"
ARTIFACT_DIR="${RUN_ROOT}/artifacts"
META_DIR="${RUN_ROOT}/meta"
STDOUT_LOG="${LOG_DIR}/cargo.stdout.log"
STDERR_LOG="${LOG_DIR}/cargo.stderr.log"
REPORT_JSON="${ARTIFACT_DIR}/shadow_report.json"
REPLAY_SH="${ARTIFACT_DIR}/replay.sh"
SUMMARY_TXT="${META_DIR}/summary.txt"
MANIFEST_JSON="${META_DIR}/artifact_manifest.json"
COMMAND_TXT="${META_DIR}/command.txt"
STATUS=0

mkdir -p "${LOG_DIR}" "${ARTIFACT_DIR}" "${META_DIR}"
cd "${ROOT_DIR}"

require_command() {
  local command="$1"
  local hint="$2"
  if ! command -v "${command}" >/dev/null 2>&1; then
    echo "[runtime-shadow] missing required command: ${command} (${hint})" >&2
    exit 2
  fi
}

require_command "rch" "install or configure remote_compilation_helper"
require_command "jq" "install jq"
require_command "python3" "install Python 3"

CMD=(
  rch exec --
  env
  "CARGO_TARGET_DIR=/tmp/rch_target_runtime_shadow_compare"
  "FTUI_RUNTIME_SHADOW_SCENARIO=${SCENARIO}"
  "FTUI_RUNTIME_SHADOW_EMIT_REPORT=1"
  cargo
  test
  -p
  ftui-runtime
  --test
  shadow_run_comparator
  shadow_runtime_operator_artifacts
  --
  --exact
  --nocapture
)

printf '%q ' "${CMD[@]}" > "${COMMAND_TXT}"
printf '\n' >> "${COMMAND_TXT}"

if "${CMD[@]}" >"${STDOUT_LOG}" 2>"${STDERR_LOG}"; then
  STATUS=0
else
  STATUS=$?
fi

REPORT_LINE="$(
  grep -h '^FTUI_RUNTIME_SHADOW_REPORT_JSON=' "${STDOUT_LOG}" "${STDERR_LOG}" | tail -n 1 || true
)"
if [[ -z "${REPORT_LINE}" ]]; then
  echo "[runtime-shadow] comparator did not emit FTUI_RUNTIME_SHADOW_REPORT_JSON" >&2
  tail -n 80 "${STDOUT_LOG}" >&2 || true
  tail -n 80 "${STDERR_LOG}" >&2 || true
  if [[ "${STATUS}" -eq 0 ]]; then
    exit 1
  fi
  exit "${STATUS}"
fi

REPORT_PAYLOAD="${REPORT_LINE#FTUI_RUNTIME_SHADOW_REPORT_JSON=}"
printf '%s\n' "${REPORT_PAYLOAD}" | jq '.' > "${REPORT_JSON}"

cat > "${REPLAY_SH}" <<EOF
#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ROOT_DIR}"
SCENARIO="\${1:-${SCENARIO}}"
RUN_ROOT="\${2:-/tmp/ftui_runtime_shadow_replay/\$(date -u +%Y%m%dT%H%M%SZ)}"

cd "\${ROOT_DIR}"
"${ROOT_DIR}/scripts/runtime_shadow_compare.sh" "\${RUN_ROOT}" "\${SCENARIO}"
EOF
chmod +x "${REPLAY_SH}"

python3 - <<'PY' "${REPORT_JSON}" "${REPLAY_SH}" "${STDOUT_LOG}" "${STDERR_LOG}" "${COMMAND_TXT}" "${MANIFEST_JSON}"
from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

report_json = Path(sys.argv[1])
replay_sh = Path(sys.argv[2])
stdout_log = Path(sys.argv[3])
stderr_log = Path(sys.argv[4])
command_txt = Path(sys.argv[5])
manifest_json = Path(sys.argv[6])

artifacts = [report_json, replay_sh, stdout_log, stderr_log, command_txt]
entries = []
for path in artifacts:
    payload = path.read_bytes()
    entries.append(
        {
            "path": str(path),
            "size_bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }
    )

manifest_json.write_text(
    json.dumps(
        {
            "artifact_count": len(entries),
            "artifacts": entries,
        },
        indent=2,
    )
    + "\n",
    encoding="utf-8",
)
PY

{
  echo "status=${STATUS}"
  echo "scenario=${SCENARIO}"
  echo "run_root=${RUN_ROOT}"
  echo "report_json=${REPORT_JSON}"
  echo "replay_sh=${REPLAY_SH}"
  echo "manifest_json=${MANIFEST_JSON}"
  echo "stdout_log=${STDOUT_LOG}"
  echo "stderr_log=${STDERR_LOG}"
  jq -r '.summary | "total_scenarios=\(.total_scenarios)\nmatched_scenarios=\(.matched_scenarios)\ndiverged_scenarios=\(.diverged_scenarios)\ntotal_mismatches=\(.total_mismatches)\ntotal_blocking_mismatches=\(.total_blocking_mismatches)"' "${REPORT_JSON}"
  jq -r '.summary.contract_coverage[] | "contract_coverage_\(.contract)=\(.covered)"' "${REPORT_JSON}"
} > "${SUMMARY_TXT}"

cat "${SUMMARY_TXT}"
exit "${STATUS}"

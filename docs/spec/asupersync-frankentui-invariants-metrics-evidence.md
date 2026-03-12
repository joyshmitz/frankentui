# Asupersync FrankenTUI Invariants, Metrics, and Evidence Schema

Bead: `bd-1sder`

## Purpose

This document defines the canonical:

- correctness invariants that targeted Asupersync migration work must preserve
- user-visible and operator-visible metrics that later implementation and rollout work must measure
- evidence schema shape that runtime, doctor, and validation tooling must emit

It is intended to be the traceability backbone for later executor, rollout, validation, and `doctor_frankentui`
beads. If a future bead cannot point back to a named invariant, metric, or evidence field here, it is underspecified.

## Scope

This contract applies only to the targeted Asupersync adoption boundary locked by
`docs/adr/ADR-010-asupersync-targeted-adoption.md`:

- `ftui-runtime` orchestration seams
- `doctor_frankentui` orchestration and evidence seams
- shadow-run, fallback, and rollout validation work for those seams

It does not redefine render-kernel correctness, widget semantics, or layout/text behavior outside the orchestration
lane.

## Canonical Invariants

Every later bead in this migration track must explicitly name which invariants it preserves, extends, or measures.

| Invariant ID | Name | Applies to | Definition | Why users/operators care |
| --- | --- | --- | --- | --- |
| `INV-RT-SEMANTIC-PARITY` | Semantic parity | `ftui-runtime`, `doctor_frankentui` | Equivalent inputs and capability context must yield equivalent externally visible outcomes before and after migration. | Avoids behavior drift hidden behind "internal refactor" language. |
| `INV-RT-DETERMINISM` | Deterministic output | `ftui-runtime` | Fixed input streams, clocks, and capability profiles must reproduce the same message ordering, evidence events, and terminal-visible state. | Enables replay, triage, and trustworthy regression comparison. |
| `INV-RT-SHUTDOWN-BOUND` | Bounded shutdown | `ftui-runtime`, `doctor_frankentui` | Shutdown, cancellation, and stop paths must complete within configured bounds and emit explicit timeout/fallback evidence when they do not. | Users care about exit reliability; operators care about stuck-task diagnosis. |
| `INV-RT-TERM-SAFETY` | Terminal safety | `ftui-runtime` | One-writer rule, screen-mode correctness, and RAII cleanup remain intact under all execution lanes. | Terminal corruption is a release blocker. |
| `INV-RT-REPLAY-IDENTITY` | Replay identity | `ftui-runtime`, validation | Replayed runs must preserve run/correlation identity and evidence lineage strongly enough to compare legacy and Asupersync lanes. | Makes shadow mode and postmortem comparison credible. |
| `INV-RT-CANCEL-VISIBILITY` | Visible cancellation semantics | `ftui-runtime`, `doctor_frankentui` | Cancellations, deadlines, detach paths, and kill decisions must be observable in evidence, not inferred from missing output. | Missing cancellation evidence creates ambiguous failures. |
| `INV-DR-ARTIFACT-COMPLETE` | Artifact completeness | `doctor_frankentui` | Required artifacts, checksums, manifests, and summaries must be present or explicitly marked missing with reason. | Go/no-go decisions depend on complete evidence bundles. |
| `INV-DR-FAILURE-CLARITY` | Failure diagnosability | `doctor_frankentui`, validation | Every failed run must retain enough structured context to identify failing step, command, timing, expected vs actual outcome, and artifact locations without rerun. | Shortens operator debug loops. |
| `INV-ROLLOUT-SHADOW-COMPARABLE` | Shadow comparability | rollout/validation | Shadow mode must emit enough shared identifiers and normalized metrics to compare legacy and Asupersync paths directly. | Prevents "shadow mode" from degenerating into uncorrelated logs. |
| `INV-ROLLOUT-FALLBACK-EXPLICIT` | Explicit fallback semantics | rollout/runtime/doctor | Fallback activation, rollback reason, and active execution lane must be machine-visible in evidence and summaries. | Operators must know which lane actually ran. |

## User-Visible And Operator-Visible Metrics

Metrics must be meaningful to humans first, not just to dashboards. Internal counters are useful only when they
support one of the metrics below.

### Runtime Metrics

| Metric ID | Metric | Unit | Surface | Why it matters |
| --- | --- | --- | --- | --- |
| `MET-RT-EFFECT-TURNAROUND-P50/P95/P99` | Time from effect scheduling to completion/failure/cancellation | milliseconds | `ftui-runtime` | Measures responsiveness impact of background work. |
| `MET-RT-SUBSCRIPTION-STOP-LATENCY-P50/P95/P99` | Time from subscription stop request to joined/terminated state | milliseconds | `ftui-runtime` | Captures shutdown cleanliness. |
| `MET-RT-PROCESS-CANCEL-LATENCY-P50/P95/P99` | Time from child-process cancel/kill request to terminal state reached | milliseconds | `ftui-runtime` | Measures orphan-process risk and operator confidence. |
| `MET-RT-SHUTDOWN-LATENCY-P50/P95/P99` | Time from shutdown initiation to terminal restoration + runtime exit | milliseconds | `ftui-runtime` | Direct user-facing exit quality metric. |
| `MET-RT-DEADLINE-BREACH-RATE` | Share/count of operations breaching configured deadlines | count, ratio | `ftui-runtime` | Indicates whether the new lane actually improves bounded execution. |
| `MET-RT-EVIDENCE-DROP-RATE` | Missing/failed evidence events relative to required events | count, ratio | runtime + validation | Protects diagnosability and replayability. |

### doctor_frankentui Metrics

| Metric ID | Metric | Unit | Surface | Why it matters |
| --- | --- | --- | --- | --- |
| `MET-DR-COMMAND-COMPLETE-P50/P95/P99` | Time for doctor/capture/suite/report commands to complete | milliseconds | `doctor_frankentui` | Measures operator workflow latency. |
| `MET-DR-RETRY-COUNT` | Retries consumed before success/failure | count | `doctor_frankentui` | Distinguishes transient recovery from chronic instability. |
| `MET-DR-ARTIFACT-COMPLETENESS-RATE` | Required artifacts present vs expected | ratio | `doctor_frankentui` | Core go/no-go signal for evidence quality. |
| `MET-DR-FAILURE-TIME-TO-TRIAGE` | Time to assemble actionable failure bundle | milliseconds | doctor + scripts | Measures postmortem efficiency, not just command success. |
| `MET-DR-REPLAY-SUCCESS-RATE` | Replays that can be performed from retained artifacts without regeneration | ratio | doctor + triage scripts | Captures practical usefulness of evidence bundles. |

### Rollout / Shadow Metrics

| Metric ID | Metric | Unit | Surface | Why it matters |
| --- | --- | --- | --- | --- |
| `MET-RO-SHADOW-DIVERGENCE-RATE` | Shadow comparisons that differ on invariant-bearing outputs | ratio | rollout/validation | Primary migration readiness metric. |
| `MET-RO-FALLBACK-ACTIVATION-RATE` | Runs that activate fallback or rollback | ratio | rollout/runtime/doctor | Indicates stability and rollout risk. |
| `MET-RO-DIAGNOSABLE-FAILURE-RATE` | Failed runs with complete structured evidence bundle | ratio | validation/rollout | Failure without evidence is operationally equivalent to a flaky system. |

## Mapping To Existing Runtime Metrics And Schema Facilities

The repo already contains metric and schema primitives that later beads should extend rather than replace:

- `crates/ftui-runtime/src/metrics_registry.rs`
  - existing counter families like `EffectsCommandTotal`, `EffectsSubscriptionTotal`, `SloBreachesTotal`,
    `RuntimeMessagesProcessedTotal`, and `TraceCompatFailuresTotal`
- `crates/ftui-runtime/src/schema_compat.rs`
  - existing schema kinds and versioning discipline for `Evidence`, `RenderTrace`, `EventTrace`, `GoldenTrace`,
    `Telemetry`, and `MigrationIr`

This bead does not require immediate code changes to those enums. It defines the canonical semantic contract that later
implementation beads must map onto those facilities.

## Canonical Evidence Schema Contract

Future work may use multiple physical files or schema kinds, but all evidence emitted for this migration lane must obey
the same logical contract.

### Required Identity Fields

Every evidence record in this lane must carry:

- `schema_kind`
- `schema_version`
- `event_type`
- `timestamp_utc`
- `run_id`
- `correlation_id`
- `lane`
  - one of `legacy`, `asupersync`, `shadow`
- `component`
  - e.g. `ftui-runtime`, `doctor_frankentui`, `validation-script`
- `operation`
  - human-meaningful unit such as `effect_execute`, `subscription_stop`, `process_cancel`, `doctor_capture`

### Required Comparison And Context Fields

When applicable, records must also carry:

- `parent_correlation_id`
- `case_id`
- `step_id`
- `capability_profile`
- `deadline_ms`
- `timeout_ms`
- `outcome`
  - e.g. `ok`, `timeout`, `cancelled`, `failed`, `fallback_activated`, `shadow_divergence`
- `reason_code`
  - stable machine-readable reason, not just free text
- `duration_ms`
- `expected`
- `actual`
- `artifact_refs`
  - manifest-friendly references to logs, traces, reports, screenshots, or replay inputs

### Required Shadow/Fallback Fields

Any shadow or fallback record must include:

- `primary_lane`
- `comparison_lane`
- `divergence_class`
  - `none`, `semantic`, `timing`, `artifact`, `schema`, `fallback`
- `fallback_trigger`
- `fallback_decision`
- `rollback_required`

### Required Integrity Fields

Required for any record that references persisted output:

- `artifact_hashes`
- `env_hash`
- `stdout_sha256` / `stderr_sha256` when command output exists
- `schema_compat`
  - compatibility outcome if the record is consumed across versions

## Canonical Correlation Rules

The migration lane must use stable identity semantics across runtime, doctor, and script layers:

1. `run_id`
   - identifies one top-level execution attempt and remains stable across all records in that attempt
2. `correlation_id`
   - identifies one operation or sub-operation within the run
3. `parent_correlation_id`
   - links nested work such as a doctor capture step that spawns a runtime process
4. `case_id`
   - identifies a named validation scenario or failure case when applicable
5. `step_id`
   - identifies a workflow step within a case or command sequence

Rules:

- `correlation_id` values must be unique within a `run_id`
- nested operations must point back to the parent operation rather than inventing disconnected ids
- shadow-mode comparisons must share a common `run_id` and use lane-specific correlation ids so paired events can be
  compared deterministically
- summaries and manifests must point back to the raw event stream by `run_id`

## Evidence Event Categories

Later implementation beads do not have to use these exact strings immediately, but they must map to these categories.

| Category | Purpose | Example events |
| --- | --- | --- |
| lifecycle | Top-level run and lane transitions | `run_start`, `run_end`, `lane_selected`, `fallback_activated` |
| effect | Runtime effect execution | `effect_scheduled`, `effect_started`, `effect_completed`, `effect_cancelled` |
| subscription | Subscription reconciliation and stop/join paths | `subscription_started`, `subscription_stop_requested`, `subscription_joined` |
| process | Child-process supervision | `process_spawned`, `process_timeout`, `process_killed`, `process_exited` |
| evidence | Artifact and manifest production | `artifact_written`, `manifest_finalized`, `schema_compat_checked` |
| validation | Shadow and replay comparison outcomes | `shadow_compare`, `replay_compare`, `divergence_detected` |
| doctor | Doctor workflow stages | `doctor_check_started`, `capture_step_end`, `suite_summary`, `report_generated` |

## Minimum Go/No-Go Evidence Bundle

Any rollout or validation gate for this migration must retain, at minimum:

- raw evidence event stream (JSONL or equivalent) with `run_id` and `correlation_id`
- summary artifact with pass/fail/divergence counts
- artifact manifest with stable keys, hashes, and paths
- explicit lane/fallback/shadow metadata
- timing data for the metrics in scope
- expected vs actual payloads for any failed or divergent case

If any part of this bundle is missing, the run is not considered fully diagnosable.

## Existing Repo Contracts This Spec Must Align With

- `crates/ftui-runtime/src/schema_compat.rs`
  - evidence and trace schema versioning discipline
- `crates/ftui-runtime/src/metrics_registry.rs`
  - built-in metric families and compatibility-failure accounting
- `crates/doctor_frankentui/coverage/e2e_jsonl_schema.json`
  - existing `run_id`, `correlation_id`, `artifact_hashes`, `expected`, and `actual` fields
- `crates/doctor_frankentui/TEST_MATRIX.md`
  - artifact completeness and failure-diagnosability expectations
- `scripts/doctor_frankentui_happy_e2e.sh`
- `scripts/doctor_frankentui_failure_e2e.sh`
- `scripts/doctor_frankentui_determinism_soak.sh`
- `scripts/doctor_frankentui_replay_triage.py`

## How Later Beads Should Use This Document

- `bd-3f24s`
  - map feature flags, shadow mode, and fallback topology onto the `lane`, `fallback_*`, and comparison fields above
- `bd-zkalo` / `bd-2vb3o` / `bd-392ka`
  - emit runtime executor evidence and timing data against the runtime invariants and metrics above
- `bd-1dccp`
  - align doctor orchestration records and manifests with the same `run_id` / `correlation_id` / artifact contract
- validation and rollout beads
  - treat missing evidence fields as contract failures, not documentation drift

## Validation Notes

This contract was derived from current repo evidence and schema surfaces in:

- `docs/spec/asupersync-frankentui-seam-inventory.md`
- `docs/adr/ADR-010-asupersync-targeted-adoption.md`
- `crates/ftui-runtime/src/metrics_registry.rs`
- `crates/ftui-runtime/src/schema_compat.rs`
- `crates/doctor_frankentui/coverage/e2e_jsonl_schema.json`
- `crates/doctor_frankentui/TEST_MATRIX.md`

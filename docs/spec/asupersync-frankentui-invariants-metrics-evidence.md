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
| `MET-RT-DEGRADED-DUTY-CYCLE` | Share of runtime spent in `stressed` or `degraded` mode | ratio | runtime + validation | Distinguishes brief pressure from chronic service degradation. |
| `MET-RT-RECOVERY-LATENCY-P50/P95/P99` | Time from pressure relief to healthy-mode restoration | milliseconds | runtime + validation | Measures whether the system actually returns to normal instead of remaining sticky. |
| `MET-RT-STRICT-BEHAVIOR-VIOLATION-RATE` | Violations of guarantees that must remain strict under load | count, ratio | runtime + validation | Separates acceptable fidelity loss from contract-breaking behavior drift. |

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

## Runtime Mode, Responsiveness, and Recovery Contract

This section is the canonical `bd-8vstx` contract. Later queue/admission, governor,
comparator, non-interference, and rollout beads must satisfy it even if the internal
control policy changes.

### Runtime Modes

| Mode | Entry condition | User-visible semantics | Operator-visible evidence | Exit rule |
| --- | --- | --- | --- | --- |
| `healthy` | No sustained pressure and no active fallback | Interactive work, rendering, and shutdown behave within the steady-state envelope; no explicit loss of fidelity is active. | Evidence shows `runtime_mode=healthy` and no active degradation or fallback interval. | Leave only when pressure exceeds the steady-state envelope or a safety invariant is threatened. |
| `stressed` | Pressure is elevated but strict guarantees are still being met without explicit shedding | Short-term batching/coalescing is allowed, but the UI must still feel responsive and no user-visible ambiguity is allowed. | Emit an early warning transition with pressure class, reason code, and the strict guarantees being preserved. | Return to `healthy` if pressure subsides before shedding begins, or advance to `degraded` if bounded fallback is required. |
| `degraded` | Explicit shedding/fallback is active to protect responsiveness or safety | Visual fidelity, batching policy, and non-critical work may degrade only according to declared policy; the runtime must remain understandable rather than merely "still running." | Emit a mode transition, active degradation level, work disposition, and recovery target through logs/evidence and any operator summary surfaces. | Leave only through an explicit `recovered` transition or fail-fast if strict guarantees can no longer be preserved. |
| `recovered` | Pressure has subsided after a stressed/degraded interval | Fidelity may be restored stepwise, but the return to normal must be explicit and non-flapping; no silent snap-back is allowed. | Emit the closing transition for the degraded interval, including recovery latency and what happened to deferred/coalesced work. | Return to `healthy` only after the configured hysteresis window and backlog drain complete. |

### Strict Behaviors That Must Hold In Every Mode

- Terminal safety, screen-mode correctness, and RAII cleanup remain strict.
- Accepted input must preserve ordering, explicit cancellation, and comprehensible outcomes even when coalescing is active.
- Shutdown, cancellation, and detach paths remain bounded and visible.
- Evidence continuity remains strict: if work is dropped, deferred, or coalesced, the operator can tell which happened and why.
- If any strict guarantee cannot be preserved, the correct action is fail-fast, not optimistic degradation.

### Behaviors That May Degrade Only With Explicit Policy

- Visual fidelity tiers, animation cadence, and expensive embellishments.
- Batching/coalescing frequency for non-critical updates.
- Throughput of background or non-interactive work.
- Detail level of diagnostics exposed to the user, provided operator-grade evidence remains intact.

### Observable Signals

| Surface | Minimum required signal | What it must answer |
| --- | --- | --- |
| Runtime evidence / telemetry | Mode transitions, degradation level, pressure class, reason code, strict guarantees preserved, work disposition | "What mode was the runtime in, why did it change, and what was allowed to degrade?" |
| `doctor_frankentui` summaries and manifests | Active degraded/fallback interval, dominant reason code, recovery outcome, artifact links | "Did this run degrade, and can I diagnose it without raw log spelunking?" |
| UI-facing hooks (HUD/status line) when available | Current mode, visible degradation tier, queue occupancy versus cap (or explicit uncapped state), active coalescing state, and work disposition | "What should the user expect right now?" |
| Validation artifacts | Mode timeline, transition checksum, strict-guarantee report | "Did this run obey the contract, and is it reproducible?" |

### Scenario Classes

| Scenario class | Purpose | Contract expectation | Canonical fixtures |
| --- | --- | --- | --- |
| Normal-path | Prove the runtime stays in `healthy` mode during ordinary work | No sustained degraded interval; brief `stressed` transitions are allowed only if they self-clear without user-visible ambiguity. | `control_idle_runtime`, `runtime_shutdown_determinism` |
| Challenge-path | Prove the runtime degrades intentionally and recovers cleanly under real pressure | `stressed` and `degraded` are allowed, but strict guarantees remain intact and recovery is explicit. | `challenge_input_flood`, `challenge_mixed_workload` |
| Negative-control | Prove the runtime does not invent degradation without pressure | No fallback, no recovery churn, and no contract signals that suggest phantom pressure. | `control_idle_runtime` |

### Recovery Expectations

1. Entering `degraded` mode must emit the reason code, active degradation level, preserved guarantees, and recovery target no later than the first degraded interval.
2. Recovery must be hysteretic and explicit; oscillating `healthy`/`degraded` state on single-sample noise is a contract violation.
3. Deferred, coalesced, or dropped work must be counted and reason-coded so operators can distinguish preserved throughput from silent loss.
4. A run is not fully recovered until the runtime returns to `healthy`, backlog drains within policy, and the closing evidence identifies the degraded interval's duration.

## Queueing Envelope and Admission/Coalescing Policy

This section is the canonical `bd-cu54l` contract. It turns the runtime-mode
contract above into an explicit control policy that later governor and
scheduling beads must implement rather than reinterpret.

### Required Measured Inputs

The controller must derive its decision from already-existing runtime signals,
not from a fresh set of undocumented heuristics:

- effect-queue telemetry: `enqueued`, `processed`, `dropped`, `high_water`, `in_flight`
- latency metrics: `MET-RT-EFFECT-TURNAROUND-*`, `MET-RT-SUBSCRIPTION-STOP-LATENCY-*`, `MET-RT-PROCESS-CANCEL-LATENCY-*`, `MET-RT-SHUTDOWN-LATENCY-*`
- pressure metrics: `MET-RT-DEADLINE-BREACH-RATE`, `MET-RT-DEGRADED-DUTY-CYCLE`, `MET-RT-RECOVERY-LATENCY-*`, `MET-RT-STRICT-BEHAVIOR-VIOLATION-RATE`
- resize evidence: coalescer regime, pending window age, forced-deadline applies, and latest-wins state
- frame-budget evidence: current degradation tier plus recent `budget_decision` history

### Envelope Classes

| Envelope | Entry signals | Required runtime mode | Required posture |
| --- | --- | --- | --- |
| `steady_state` | Strict latency metrics are within configured bounds; no queue drops; no active degradation; and, when a queue cap exists, `in_flight < 0.5 * max_queue_depth`. | `healthy` | Admit all work normally; do not invent degradation or backlog shedding. |
| `soft_overload` | Strict guarantees still hold, but pressure is visible: queue occupancy reaches the stressed watermark, the resize coalescer enters burst mode, forced applies begin to appear, or budget pressure becomes sustained. | `stressed` | Preserve strict work, start bounded coalescing, and slow background admission before user-visible ambiguity appears. |
| `hard_overload` | Any of: queue backpressure/drop fires, `in_flight >= 0.8 * max_queue_depth`, deadline-breach budget is exceeded, user-visible degradation is active, or recovery from a previous degraded interval is still incomplete. | `degraded` | Preserve strict work, defer background work, and allow only declared fidelity loss and bounded coalescing. |
| `unsafe` | Terminal safety, bounded shutdown, evidence continuity, or another strict guarantee can no longer be preserved with the current workload. | `n/a` (terminal condition) | Stop optimistic degradation and surface an explicit failure with evidence. |

Notes:

- The stressed watermark is `0.5 * max_queue_depth`, the degraded watermark is
  `0.8 * max_queue_depth`, and the recovery watermark is `0.25 * max_queue_depth`.
- If `max_queue_depth == 0` (unbounded queue), mode changes must be driven by
  latency, deadline, and degradation evidence rather than raw depth alone.
- `unsafe` is not a steady-state runtime mode. It is the terminal condition
  that requires fail-fast from the current mode when a strict guarantee would
  otherwise be violated.

### Work Classes And Allowed Disposition

| Work class | Examples | `healthy` | `stressed` | `degraded` |
| --- | --- | --- | --- | --- |
| `strict_interactive` | accepted input ordering, active cursor/focus updates, terminal restore, cancellation, shutdown, evidence rows required to explain a decision | Admit immediately. | Admit immediately; never drop; only bounded coalescing already covered by a declared contract is allowed. | Admit immediately; if this cannot be preserved, fail-fast. |
| `visible_coalescible` | resize-driven rerenders, non-essential refreshes, HUD updates, optional trace payload generation | Admit normally. | Coalesce with hard deadlines and explicit reason codes. | Coalesce aggressively, allow fidelity loss, and keep latest-wins semantics explicit. |
| `background_deferrable` | queued `Cmd::Task`, low-priority subscription work, artifact post-processing, shadow helpers | Admit while within steady-state envelope. | Admit only while projected queue occupancy stays below the degraded watermark when capped, or below the controller's latency-derived admission threshold when uncapped, and strict latencies remain healthy. | Defer until recovery unless needed to preserve a strict guarantee. |
| `best_effort_droppable` | optional sampling, verbose diagnostics expansion, speculative analysis, advisory summaries | Admit opportunistically. | Drop first when pressure rises; emit reason-coded evidence. | Drop by default; re-enable only during recovery. |

### Canonical Control Rules

1. `healthy` mode is the no-surprises baseline: all work classes are admitted,
   queue drops are forbidden, and resize/frame controllers may optimize locally
   but must not claim degraded service.
2. `stressed` mode is an early intervention band, not permission for vague
   slowdowns. The controller may coalesce `visible_coalescible` work and defer
   `background_deferrable` work, but it must keep strict latency metrics within
   budget and preserve comprehensible user behavior.
3. `degraded` mode is the only band where explicit shedding is allowed. In this
   band, `strict_interactive` work remains strict, `visible_coalescible` work
   follows bounded coalescing plus existing degradation tiers, and
   `best_effort_droppable` work is dropped with explicit evidence.
4. Recovery is staged. Deferred work may be re-admitted only after three
   consecutive control intervals with no new drops, no new forced-deadline
   applies, queue occupancy below the recovery watermark when capped or below
   the latency-derived recovery threshold when uncapped, and the frame budget
   back at the baseline tier.
5. `recovered` is a real mode, not a log flourish. It closes the interval,
   reports recovery latency, and records what happened to deferred, coalesced,
   and dropped work before the runtime returns to `healthy`.

### Verification Requirements For Later Implementation Beads

- unit tests must cover steady/stressed/degraded/unsafe classification,
  watermark behavior, and the `max_queue_depth == 0` case
- stress scripts must cover `input_backpressure`, `mixed_workload`,
  `shutdown_pressure`, and a negative-control idle run
- evidence and HUD surfaces must show `runtime_mode`, `pressure_class`,
  queue occupancy versus cap (or an explicit uncapped state), active
  coalescing state, and deferred/coalesced/dropped work counts whenever the
  runtime is not `healthy`

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

### Required Runtime-Mode Fields

Any record that enters, exits, or reports `stressed`, `degraded`, or `recovered`
behavior must include:

- `runtime_mode`
- `runtime_mode_before`
- `runtime_mode_after`
- `pressure_class`
- `degradation_level`
- `queue_depth`
- `queue_capacity`
- `queue_high_water`
- `coalescing_state`
- `coalesced_count`
- `deferred_count`
- `dropped_count`
- `reason_code`
- `strict_guarantees`
  - machine-readable list of guarantees still being held strict
- `degraded_behaviors`
  - machine-readable list of behaviors currently allowed to degrade
- `recovery_target`
- `signal_surface`
  - e.g. `otel`, `evidence_jsonl`, `doctor_summary`, `hud`
- `work_disposition`
  - `preserved`, `deferred`, `coalesced`, `dropped_with_reason`, or `failed_fast`

Any record that closes a degraded interval must also include:

- `recovery_completed`
- `recovery_latency_ms`
- `degraded_interval_ms`

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
- `bd-8vstx`
  - use the runtime-mode contract above as the user/operator definition of degraded service and recovery
- `bd-td5el` / `bd-eh85k`
  - implement load-governor and runtime scheduling changes so they produce the mode, signal, and recovery evidence above
- `bd-lu69j` / `bd-cn7eq`
  - treat missing mode/recovery evidence or strict-behavior violations as rollout blockers rather than "nice to have" telemetry
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

# Asupersync FrankenTUI Dependency Boundaries, Flags, Shadow Mode, and Fallback

Bead: `bd-3f24s`

## Purpose

This document defines how the targeted Asupersync migration is enabled, evaluated, disabled, and rolled back.

It turns the architectural boundary from `ADR-010` and the invariant/evidence contract from
`asupersync-frankentui-invariants-metrics-evidence.md` into an operational activation plan.

The design goals are:

- keep Asupersync dependencies out of pure crates and render hot paths
- make lane selection explicit at compile time and runtime
- support shadow comparison without ambiguous behavior
- ensure fallback and rollback are visible, bounded, and machine-verifiable

## Dependency Boundary

### Allowed Dependency Placement

Asupersync-related dependencies should be introduced only where orchestration already lives:

- `crates/ftui-runtime`
  - internal executor seam
  - effect execution
  - subscription/process lifecycle coordination
  - evidence emission for lane selection, cancellation, and fallback
- `crates/doctor_frankentui`
  - capture/retry/wait orchestration
  - external command supervision
  - artifact/evidence assembly

Optional supporting dependencies may be added to a small internal helper crate later if needed, but only if that crate
depends downward on orchestration crates and not on pure render/layout/text/widget crates.

### Forbidden Dependency Placement

Asupersync-related dependencies must not be added to:

- `crates/ftui-render`
- `crates/ftui-layout`
- `crates/ftui-text`
- `crates/ftui-style`
- `crates/ftui-widgets`
- public facade crates merely to expose a second async-first API shape

This follows the existing workspace boundary in [Cargo.toml](/data/projects/frankentui/Cargo.toml) and the current
feature-gated runtime style in [ftui-runtime/Cargo.toml](/data/projects/frankentui/crates/ftui-runtime/Cargo.toml).

## Compile-Time Feature Strategy

### Principle

Compile-time flags decide whether the Asupersync lane is present in the binary at all.
Runtime flags decide which lane is active for a given run.

This keeps rollout reversible without forcing all builds to pay the dependency cost.

### Proposed Features

Additive features only:

- `ftui-runtime/asupersync-executor`
  - enables the internal Asupersync-backed executor implementation
- `doctor_frankentui/asupersync-orchestration`
  - enables Asupersync-backed orchestration for doctor flows
- optional facade forwarding later:
  - `ftui/asupersync`
    - forwards to runtime feature only if a public packaging convenience is needed

### Rules

- default features remain unchanged
- no existing feature should silently switch semantics when the new feature is enabled
- the new features must compile out completely when disabled
- tests that target the Asupersync lane should require the explicit feature, the same way `event-trace` and
  `telemetry` tests already require explicit features in `ftui-runtime`

## Runtime Selection Surface

Runtime selection must be explicit and machine-visible.

### Canonical Lane Names

- `legacy`
  - current non-Asupersync path
- `asupersync`
  - active Asupersync-backed path
- `shadow`
  - legacy remains authoritative while Asupersync executes in comparison mode

These lane names must match the evidence contract from
`docs/spec/asupersync-frankentui-invariants-metrics-evidence.md`.

### Selection Policy

The runtime/doctor selection surface should support:

- explicit configuration value
  - example shape: `legacy | shadow | asupersync`
- explicit environment override for CI and rollback drills
- explicit summary/evidence output naming the active lane

The system must not:

- auto-promote from `legacy` to `asupersync`
- silently choose `shadow` because a feature was compiled in
- silently fall back without emitting a machine-readable record

### Selection Precedence

Highest to lowest:

1. explicit operator override
2. explicit command/config selection
3. build default

If an unavailable lane is requested, the process must either:

- fail fast with a clear configuration error, or
- enter an explicitly logged fallback path that records why the request could not be honored

It must never quietly run some other lane and pretend the requested one was active.

## Shadow-Run Topology

Shadow mode should copy the repo’s existing tested pattern from the policy-as-data work in
`crates/ftui-runtime/tests/policy_e2e.rs` and
`crates/ftui-runtime/tests/e2e_recipe_f_policy_as_data.rs`:

- one lane remains authoritative for user-visible behavior
- the comparison lane runs in parallel or side-by-side at the decision/effect level
- divergences are logged with shared run identity and lane-specific correlation

### Runtime Shadow Mode

Authoritative path:

- `legacy` remains user-visible and owns actual side effects that would mutate terminal or process state

Comparison path:

- Asupersync executor runs the same logical work in shadow mode where safe
- comparison records are emitted for:
  - effect scheduling/completion/cancellation
  - subscription start/stop/join timing
  - child-process supervision decisions
  - shutdown sequencing outcomes

Constraints:

- shadow mode must not violate the one-writer rule
- shadow mode must not duplicate external side effects that cannot be safely mirrored
- where double execution would be unsafe, shadow mode compares normalized plans/decisions instead of executing both
  lanes destructively

### doctor_frankentui Shadow Mode

Authoritative path:

- legacy doctor execution remains the source of truth for exit codes and produced artifacts during early rollout

Comparison path:

- Asupersync lane emits comparison evidence for retries, waits, timeout handling, and artifact accounting
- shared `run_id` links both lanes
- lane-local `correlation_id`s allow deterministic pairing

Where duplicate process execution would be expensive or unsafe, shadow mode may compare:

- step plans
- timeout decisions
- retry schedules
- manifest completeness

rather than blindly rerunning external tools twice.

## Fallback Topology

Fallback must be explicit, bounded, and staged.

### Fallback Triggers

Allowed triggers include:

- requested lane not compiled in
- configuration or capability validation failure
- schema compatibility failure
- startup self-check failure for the Asupersync lane
- runtime invariant breach that invalidates continued comparison or execution

### Fallback Actions

For both runtime and doctor, fallback should be:

1. detect trigger
2. emit evidence with `fallback_trigger`, `fallback_decision`, `reason_code`, and active lane
3. switch to `legacy` if safe to do so
4. mark the run summary so operators can see the fallback without inspecting raw logs

Fallback is valid only if the system can still preserve:

- terminal safety
- bounded shutdown
- artifact completeness
- understandable operator diagnostics

If those cannot be preserved, the correct action is fail-fast, not optimistic fallback.

### Runtime Fallback Levels

- `runtime_fallback_level=0`
  - no fallback active
- `runtime_fallback_level=1`
  - Asupersync requested but startup validation forced legacy
- `runtime_fallback_level=2`
  - shadow comparison disabled mid-run, legacy remains authoritative
- `runtime_fallback_level=3`
  - fatal invariant breach, fail-fast required

### doctor_frankentui Fallback Levels

- `doctor_fallback_level=0`
  - no fallback active
- `doctor_fallback_level=1`
  - orchestration lane downgraded to legacy before expensive work begins
- `doctor_fallback_level=2`
  - comparison/reporting degraded but core command completed
- `doctor_fallback_level=3`
  - evidence completeness or control-plane safety lost; command fails explicitly

## Rollback Policy

Rollback must be operationally trivial:

- disable runtime lane by forcing `legacy`
- disable doctor lane by forcing `legacy`
- if necessary, rebuild without `asupersync-executor` and `asupersync-orchestration`

Rollback drills must verify:

- active lane is visible in summaries and evidence
- no stale config keeps trying to reactivate the disabled lane
- legacy behavior remains intact after rollback
- shadow/fallback evidence remains parseable after rollback

## Required Evidence For Flags, Shadow, and Fallback

Every run in this migration track must emit enough evidence to answer:

- which lane was requested?
- which lane was actually active?
- was shadow mode enabled?
- did fallback occur?
- why did fallback occur?
- which invariants were compared or waived?

Minimum fields:

- `lane`
- `primary_lane`
- `comparison_lane`
- `fallback_trigger`
- `fallback_decision`
- `rollback_required`
- `reason_code`
- `schema_compat`
- `run_id`
- `correlation_id`

This aligns directly with the evidence contract in
`docs/spec/asupersync-frankentui-invariants-metrics-evidence.md`.

## Recommended Configuration Shape

The exact config type can evolve, but it should be expressible as a single explicit block rather than scattered booleans.

Recommended shape:

```toml
[asupersync]
mode = "legacy"         # legacy | shadow | asupersync
shadow_compare = true   # explicit, not implied
fallback_policy = "safe_legacy"
fail_open = false
emit_shadow_artifacts = true
```

Rules:

- `mode=legacy` ignores Asupersync lane even if compiled in
- `mode=shadow` requires comparison evidence
- `mode=asupersync` may fall back only with explicit evidence
- `fail_open=false` is preferred for early rollout so silent degradation does not mask bugs

## Validation Surfaces This Plan Must Feed

Future implementation must preserve and extend:

- `cargo test -p ftui-runtime --all-targets`
- `cargo bench -p ftui-runtime --bench tick_strategy_bench`
- `cargo test -p doctor_frankentui --all-targets -- --nocapture`
- `./scripts/doctor_frankentui_happy_e2e.sh /tmp/doctor_frankentui_ci/happy`
- `./scripts/doctor_frankentui_failure_e2e.sh /tmp/doctor_frankentui_ci/failure`
- `./scripts/doctor_frankentui_determinism_soak.sh /tmp/doctor_frankentui_ci/determinism 3`
- `./scripts/doctor_frankentui_replay_triage.py --run-root /tmp/doctor_frankentui_ci/failure --max-signals 8`
- `./scripts/doctor_frankentui_coverage.sh /tmp/doctor_frankentui_ci/coverage`

Additional rollout-specific validation should prove:

- lane selection is explicit and logged
- shadow mode compares without mutating authoritative output
- fallback transitions are deterministic and machine-readable
- rollback to legacy is immediate and testable

## Guidance For Follow-On Beads

- `bd-zkalo` / `bd-2vb3o` / `bd-392ka`
  - implement the runtime executor seam behind the feature and lane boundaries defined here
- `bd-1dccp`
  - align doctor orchestration, retries, and artifact reporting with the same lane/fallback model
- validation/rollout beads
  - treat missing lane/fallback evidence as contract failures, not optional observability

## Validation Notes

This plan was grounded in:

- `Cargo.toml`
- `crates/ftui-runtime/Cargo.toml`
- `crates/doctor_frankentui/Cargo.toml`
- `crates/ftui/Cargo.toml`
- `crates/ftui-runtime/tests/policy_e2e.rs`
- `crates/ftui-runtime/tests/e2e_recipe_f_policy_as_data.rs`
- `docs/adr/ADR-010-asupersync-targeted-adoption.md`
- `docs/spec/asupersync-frankentui-invariants-metrics-evidence.md`

# Asupersync FrankenTUI Seam Inventory

Bead: `bd-3838s`

## Purpose

This document is the current source of truth for where targeted Asupersync-style capability threading and structured async orchestration help FrankenTUI, where they do not, and which existing code/tests/scripts future migration work must preserve.

The intended migration boundary is narrow:

- Yes: runtime shell orchestration, effect execution, subscription/process lifecycles, cancellation/deadline propagation, doctor workflow waiting/retry/evidence plumbing.
- No: render-kernel algorithms, pure layout/text/view logic, widget rendering semantics, or public API churn that would destabilize the facade.

## Completed Groundwork

The repo already contains groundwork that later Asupersync-focused beads should treat as prerequisites, not rediscovery work.

| Bead | Current state | Repo evidence | Why it matters |
| --- | --- | --- | --- |
| `bd-ehk.1` | closed | `ftui-core` closed bead notes in `.beads/issues.jsonl`; Cx capability context threading is recorded as implemented there | Establishes the capability-threading direction and cancellation/deadline vocabulary future work is expected to build on. |
| `bd-1q5.18` | closed | `.beads/issues.jsonl` says Ring 1 terminal I/O gained Cx-aware variants and telemetry | Confirms terminal I/O is already considered a deadline/cancellation seam, so runtime-shell work should integrate with that rather than invent a parallel mechanism. |
| `bd-37a.6` | closed | `.beads/issues.jsonl`; current runtime has `EffectQueue`, `SubscriptionManager`, `ProcessSubscription`, lifecycle tracing, and bounded shutdown waits | Means the runtime already has an effect/subscription skeleton. Future migration work should refine and bridge it, not replace it wholesale. |

## Runtime Shell Seams

These are the concrete `ftui-runtime` surfaces where targeted Asupersync integration would pay off.

| Seam | Current code surface | Current behavior | User / operator pain point addressed by migration | Guardrail |
| --- | --- | --- | --- | --- |
| Command/effect execution | `crates/ftui-runtime/src/program.rs` (`Cmd`, `EffectQueue`, `execute_cmd`) | Commands can run inline, on spawned threads, or through the queue scheduler; shutdown is bounded and detach-safe | Slow effects can still fragment cancellation behavior and complicate deadline reasoning | Preserve `Cmd` and `Model` contracts; improve internals, not the public programming model |
| Subscription lifecycle | `crates/ftui-runtime/src/subscription.rs`; `Program::reconcile_subscriptions()` in `crates/ftui-runtime/src/program.rs` | Declarative subscriptions are diffed, started/stopped, and joined with bounded waits | Subscription shutdown clarity and cancellation determinism matter for exit latency and failure diagnosis | Preserve subscription IDs and declarative reconciliation semantics |
| Process supervision | `crates/ftui-runtime/src/process_subscription.rs` | Child processes stream stdout/stderr, honor timeout/stop, and emit explicit killed/exited/error events | Operators need deterministic child-process cleanup and clear failure surfaces | No change to message delivery semantics; supervision improvements must remain observable and testable |
| Input/event polling | `Program::run_event_loop()`, `drain_ready_events()`, `poll_timeout`, resize coalescer, fairness guard in `crates/ftui-runtime/src/program.rs` | Event loop already has bounded zero-timeout draining, resize coalescing, and fairness accounting | Missed deadlines, input starvation, or shutdown lag show up as visible responsiveness problems | Keep one-writer rule and event ordering guarantees intact |
| Tick / background screen policy | `tick_strategy` plumbing in `crates/ftui-runtime/src/program.rs` and `crates/ftui-runtime/src/tick_strategy/*` | Background work can already be selectively reduced with explicit strategies | Gives a place to integrate deadline-aware scheduling without contaminating view/render code | Do not push strategy logic into widgets or render code |
| State persistence + shutdown | `on_shutdown()`, `save_state()`, `load_state()`, `check_checkpoint_save()` in `crates/ftui-runtime/src/program.rs` | Shutdown already runs model hooks, persistence flushes, subscription stop, and tick-strategy shutdown | Users care about graceful exit and state survival; operators care about bounded cleanup | Preserve shutdown ordering and replayability |
| Evidence + guardrails | `EvidenceSink`, render trace, `FrameGuardrails` in `crates/ftui-runtime/src/program.rs` | Runtime can already emit evidence and enforce memory/queue guardrails | Future orchestration work must improve diagnosability, not reduce it | Any migration must keep evidence emission machine-readable and deterministic |

## doctor_frankentui Seams

`doctor_frankentui` is already an orchestration-heavy crate. It is an obvious target for targeted async/capability work because its pain points are operational, not render-path related.

| Seam | Current code surface | Current behavior | User / operator pain point addressed by migration | Guardrail |
| --- | --- | --- | --- | --- |
| Doctor command health/smoke orchestration | `crates/doctor_frankentui/src/doctor.rs` | Runs subcommand `--help` checks, bounded capture smoke runs, degraded-mode classification, and summary output | Faster, clearer readiness checks and bounded failure handling | Preserve CLI behavior and exit-code mapping |
| Capture / replay workflow | `crates/doctor_frankentui/src/capture.rs` | Resolves capture config, runs app/capture tooling, writes `RunMeta`, ledger, logs, and output artifacts | Long-running external tooling, retries, timeouts, and evidence assembly are precisely where capability-aware orchestration helps | Preserve artifact schema and deterministic replay contract |
| Seed/demo waiting and retries | `crates/doctor_frankentui/src/seed.rs` plus seed integration tests in `TEST_MATRIX.md` | Wait/retry loops, auth forwarding, endpoint normalization, optional reservation behavior | Users need reliable setup; operators need clear retry behavior and bounded waits | Keep deterministic request sequencing and logged failure signatures |
| Suite/report aggregation | `crates/doctor_frankentui/src/suite.rs`, `src/report.rs`, `TEST_MATRIX.md`, `VERIFICATION_REPORT.md` | Multi-run orchestration already produces suite summaries, artifact maps, and evidence bundles | Rollout confidence depends on artifact completeness and reproducibility, not just pass/fail bits | Preserve stable artifact-map keys and report contract |

## doctor_frankentui Workflow Topology By Command

The broad seam table above is enough to justify the migration boundary, but
`bd-1889t` needs a more operational inventory of how each command decomposes
today.

| CLI command | Primary code path | Network/bootstrap lane | Subprocess lane | Artifact lane | Aggregation lane | Existing observable outputs |
| --- | --- | --- | --- | --- | --- | --- |
| `seed-demo` | `crates/doctor_frankentui/src/seed.rs` | MCP health polling, JSON-RPC retries, auth forwarding, endpoint normalization | none today | optional RPC log file, deterministic summary payload | none | seed summary JSON/human output, retry logs, server timeout failure text |
| `replay` / `capture` | `crates/doctor_frankentui/src/capture.rs` | optional `seed-demo` bootstrap before capture | app launch, VHS invocation, ttyd interaction, tmux observer lifecycle, timeout enforcement, final exit classification | run dir creation, `run_meta.json`, `evidence_ledger.jsonl`, media/snapshot outputs, ttyd/tmux artifact files | none at command scope | `trace_id`, `fallback_reason`, `capture_error_reason`, `ttyd_*`, `tmux_*`, output/snapshot paths, policy id |
| `certify` / `doctor` | `crates/doctor_frankentui/src/doctor.rs` | none directly beyond environment/capture stack checks | `--help` command probes, capture smoke run, optional tmux app-smoke fallback with bounded wait/kill | `doctor_summary.json`, smoke logs, tmux pane/session artifacts | single-command summary synthesis | degraded-mode classification, fallback hints, app-smoke summary, attach command, pane capture/log paths |
| `migrate` / `suite` | `crates/doctor_frankentui/src/suite.rs` | inherited from per-run capture invocations | per-profile CLI subprocess fan-out, fail-fast / keep-going behavior, report subprocess invocation | suite dir, summary JSON, manifest, report log/json/html | merges per-run `RunMeta` into suite-wide trace/fallback/error index | manifest trace ids, fallback profiles, capture-error profiles, report outputs |
| `report` | `crates/doctor_frankentui/src/report.rs` | none | none beyond file reads | HTML + JSON report generation over retained run artifacts | merges run metadata and artifact links into operator view | trace ids, fallback/capture-error rows, evidence-ledger links, tmux/log/media references |

### Current Cancellation / Timeout Boundaries

The highest-value supervision boundaries are already visible in code:

- `seed.rs`
  - server readiness loop with explicit timeout and retry backoff
  - per-call HTTP client timeout and retriable RPC/tool error classification
- `capture.rs`
  - seed bootstrap optionality and required/optional failure policy
  - child-process timeout handling via `wait_timeout`
  - fallback activation when capture aborts early or degraded paths are forced
  - tmux observer start/finalize lifecycle and keep-open vs cleanup policy
- `doctor.rs`
  - bounded smoke capture path
  - degraded-capture vs hard-failure classification
  - tmux fallback session creation, bounded wait, pane capture, conditional kill
- `suite.rs`
  - fail-fast / keep-going selection
  - per-profile process orchestration and late report generation

### Current Artifact And Correlation Contract

`doctor_frankentui` already has a concrete operator contract; later Asupersync
work must preserve it rather than "recreate observability later."

- Per-run identity and evidence live primarily in `crates/doctor_frankentui/src/runmeta.rs`.
  - `RunMeta` already carries `trace_id`, `policy_id`, `run_dir`,
    `fallback_active`, `fallback_reason`, `capture_error_reason`,
    `evidence_ledger`, `ttyd_*`, `tmux_*`, and media/snapshot paths.
- `capture.rs` emits the richest raw evidence surface.
  - `run_meta.json`
  - `evidence_ledger.jsonl`
  - optional ttyd shim/runtime logs
  - optional tmux session, pane capture, and pane log artifacts
- `doctor.rs` summarizes certification outcomes into `doctor_summary.json`.
  - includes capture-stack health, degraded-mode reason, smoke detail,
    fallback error, and tmux attach/session artifacts
- `suite.rs` preserves correlation across runs.
  - suite summary JSON and manifest expose `trace_ids`, fallback profiles,
    capture-error profiles, and per-run artifact references
- `report.rs` is the operator-facing read model over the same contract.
  - HTML/JSON reports surface trace IDs, fallback reasons, capture errors,
    evidence ledger links, and tmux/log/media artifacts

### Recommended doctor_frankentui Migration Slices

Given the current topology, the lowest-risk sequence is:

1. `seed.rs`
   - isolate readiness polling, retries, and request logging into the first
     supervised network/bootstrap lane
2. `capture.rs`
   - migrate subprocess + observer management while holding the `RunMeta` and
     evidence-ledger schema stable
3. `doctor.rs`
   - formalize degraded-mode and tmux fallback as explicit supervised outcomes
4. `suite.rs`
   - move fan-out/report orchestration after the lower-level lane semantics are
     stable enough to aggregate deterministically
5. `report.rs`
   - keep largely synchronous unless later evidence volume makes incremental or
     bounded aggregation necessary

## Existing Verification Surfaces To Preserve

Future Asupersync-focused work should extend these commands and artifacts, not replace them.

### Runtime-focused

- `cargo test -p ftui-runtime --all-targets`
- `cargo bench -p ftui-runtime --bench tick_strategy_bench`
- `crates/ftui-runtime/tests/integration_tick_strategy_lifecycle.rs`
- `crates/ftui-runtime/tests/proptest_tick_strategy_invariants.rs`

### doctor_frankentui-focused

- `cargo test -p doctor_frankentui --all-targets -- --nocapture`
- `./scripts/doctor_frankentui_happy_e2e.sh /tmp/doctor_frankentui_ci/happy`
- `./scripts/doctor_frankentui_failure_e2e.sh /tmp/doctor_frankentui_ci/failure`
- `./scripts/doctor_frankentui_determinism_soak.sh /tmp/doctor_frankentui_ci/determinism 3`
- `./scripts/doctor_frankentui_replay_triage.py --run-root /tmp/doctor_frankentui_ci/failure --max-signals 8`
- `./scripts/doctor_frankentui_coverage.sh /tmp/doctor_frankentui_ci/coverage`

### Evidence / contract documents

- `crates/doctor_frankentui/TEST_MATRIX.md`
- `crates/doctor_frankentui/VERIFICATION_REPORT.md`
- `crates/doctor_frankentui/coverage/e2e_jsonl_schema.json`

## User-Facing Motivations By Seam

This migration lane should stay grounded in user-visible or operator-visible outcomes:

- Runtime effect/subscription seams:
  - Better responsiveness under slow background work
  - Faster and more predictable shutdown
  - Cleaner child-process cancellation and fewer orphaned tasks
  - Stronger failure traces when async work misbehaves
- doctor_frankentui seams:
  - Shorter diagnosis cycles when capture/replay tooling fails
  - Bounded wait/retry behavior instead of hangs
  - Higher confidence that evidence bundles are complete and replayable
  - Clearer go/no-go signals from machine-readable artifacts

## No-Go Zones

These are explicit anti-goals for this migration track.

- Do not rewrite `ftui-render` core buffer/diff/presenter machinery.
  - Files such as `crates/ftui-render/src/buffer.rs`, `crates/ftui-render/src/diff.rs`, and `crates/ftui-render/src/presenter.rs` are hot-path correctness/perf code, not orchestration seams.
- Do not push Asupersync concerns into pure layout/text/widget crates.
  - `ftui-layout`, `ftui-text`, and most widget rendering code should remain deterministic library code without orchestration coupling.
- Do not change the public `Model` / `Cmd` / `ftui` facade shape without a separate explicit API-contract decision.
- Do not weaken terminal safety properties.
  - One-writer rule, screen-mode correctness, RAII terminal cleanup, and replay determinism stay mandatory.

## Recommended Next Beads

Based on the current repo state, the inventory supports this sequence:

1. `bd-2st19`
   - Convert this inventory into the targeted adoption ADR with public API guardrails and anti-goals.
2. `bd-1sder`
   - Formalize invariants, user-visible metrics, and a canonical evidence schema using the seams and verification surfaces named here.
3. `bd-2vb3o` / `bd-zkalo` / `bd-392ka`
   - Implement the executor seam behind existing runtime APIs instead of introducing a parallel side-effect stack.
4. `bd-1dccp`
   - Apply the same bounded orchestration/evidence discipline to `doctor_frankentui`, where the operational pain is already concrete.

## Validation Notes

This inventory was validated against current code and verification artifacts in:

- `crates/ftui-runtime/src/program.rs`
- `crates/ftui-runtime/src/subscription.rs`
- `crates/ftui-runtime/src/process_subscription.rs`
- `crates/doctor_frankentui/src/doctor.rs`
- `crates/doctor_frankentui/src/capture.rs`
- `crates/doctor_frankentui/TEST_MATRIX.md`
- `crates/doctor_frankentui/VERIFICATION_REPORT.md`

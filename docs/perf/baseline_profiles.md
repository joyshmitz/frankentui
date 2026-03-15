# Baseline Profiles & Hotspot Opportunity Matrix

> bd-2vr05.15.4.1 — Collected 2026-02-16 on Contabo VPS workers (rch fleet)

## 0. Canonical Performance Program Inventory

This document is also the canonical inventory for `bd-9v2se`:

- what FrankenTUI already exposes as measurable performance surfaces,
- which invariants later optimization beads must preserve,
- and which workloads are worth optimizing instead of merely being easy to time.

The intent is to keep later beads such as `bd-760ih`, `bd-p8i4s`, and `bd-vtor0`
anchored to current code and artifact reality rather than ad hoc profiling notes.

### 0.1 Observable Surfaces

| Surface | Current observables | Primary code / script anchors | Artifact / output anchors | Measurement mode |
|---------|----------------------|-------------------------------|---------------------------|------------------|
| Render pipeline | frame latency, diff latency, presenter latency, changed cells, emitted bytes, allocation counts | `crates/ftui-demo-showcase/src/bin/profile_sweep.rs`, `crates/ftui-demo-showcase/benches/demo_pipeline_bench.rs`, `crates/ftui-render/benches/diff_bench.rs`, `crates/ftui-render/benches/presenter_bench.rs` | `profile_sweep --json`, Criterion output, flamegraph / heaptrack captures | stage-local and end-to-end |
| Runtime orchestration | frame timing, evidence events, resize-coalescer decisions, subscription reconcile counts, fairness-guard yields, shutdown task processing | `crates/ftui-runtime/src/program.rs`, `crates/ftui-runtime/src/render_trace.rs`, `crates/ftui-runtime/src/metrics_registry.rs`, `crates/ftui-runtime/src/subscription.rs` | evidence JSONL, render trace JSONL, tracing spans, counters under `metrics_registry` | mostly end-to-end with some stage-local hooks |
| Terminal ownership / output path | write timing, flush timing, ANSI emission bytes, screen-mode transitions, one-writer routing | `crates/ftui-runtime/src/terminal_writer.rs`, `crates/ftui-core/src/terminal_session.rs`, `docs/one-writer-rule.md`, `docs/adr/ADR-005-one-writer-rule.md` | terminal IO stats, render trace, evidence events, PTY captures | end-to-end, correctness-gated |
| Pane performance | solver latency, structural operation latency, replay cost, terminal drag path cost, web pointer lifecycle cost, perf-stat counters | `crates/ftui-layout/benches/layout_bench.rs`, `crates/ftui-layout/benches/pane_profile_harness.rs`, `crates/ftui-runtime/benches/pane_terminal_bench.rs`, `crates/ftui-web/benches/pane_pointer_bench.rs`, `scripts/pane_profile.sh` | `target/pane-profiling/...`, `symbol_metadata.txt`, harness manifests/snapshots, `perf stat` text | stage-local plus workflow-level |
| doctor workflows | command completion time, retry count, artifact completeness, replay readiness, determinism divergence, coverage-gate outcomes | `scripts/doctor_frankentui_happy_e2e.sh`, `scripts/doctor_frankentui_failure_e2e.sh`, `scripts/doctor_frankentui_determinism_soak.sh`, `scripts/doctor_frankentui_replay_triage.py`, `scripts/doctor_frankentui_coverage.sh` | `meta/summary.json`, `artifact_manifest.json`, `case_results.json`, `replay_triage_report.json`, `coverage_gate_report.json` | end-to-end and artifact-level |
| Demo / PTY regression suites | suite summaries, per-case JSONL logs, trace hashes, scenario-specific summaries | `scripts/demo_showcase_e2e.sh`, `scripts/pane_e2e.sh`, `scripts/e2e_test.sh`, `docs/testing/e2e-summary-schema.md` | `summary.json`, `*.jsonl`, per-suite logs under result roots | end-to-end |

### 0.2 Non-Negotiable Invariants

These are the invariants that later performance work must preserve. They are not
"nice to have" checks that can be traded away for speed.

| Invariant class | What must remain true | Where it is grounded today |
|----------------|-----------------------|----------------------------|
| Visible output equivalence | The same deterministic inputs must yield the same buffer-visible / terminal-visible output unless a bead explicitly changes semantics. | `crates/ftui-render/tests/*`, demo snapshots, `profile_sweep` real-screen pipeline |
| One-writer terminal safety | Performance work may not bypass `TerminalWriter`, corrupt cursor state, or weaken RAII cleanup. | `docs/one-writer-rule.md`, `docs/adr/ADR-005-one-writer-rule.md`, `crates/ftui-core/src/terminal_session.rs` |
| Screen-mode correctness | Inline vs alt-screen behavior, cleanup ordering, and terminal restoration remain intact under profiling and optimization. | `crates/ftui-runtime/src/terminal_writer.rs`, `crates/ftui-core/src/terminal_session.rs`, PTY E2E scripts |
| Subscription / message-order integrity | Runtime profiling must not reorder required message delivery or hide cancellation/shutdown behavior. | `crates/ftui-runtime/src/program.rs`, `crates/ftui-runtime/src/subscription.rs`, runtime tests around shutdown + reconcile |
| Replay / artifact fidelity | doctor and E2E performance work must preserve artifact completeness and replayability, not just raw speed. | `crates/doctor_frankentui/VERIFICATION_REPORT.md`, doctor E2E scripts, determinism soak scripts |
| Deterministic evidence linkage | Runs must remain joinable by stable identifiers and artifact paths so later comparisons are auditable. | evidence sink / render trace in `ftui-runtime`, doctor summary + manifest JSON contracts |

### 0.3 Optimization-Critical Workflows

These are the workload classes that later baseline and hotspot beads should
prioritize. If a proposed optimization does not improve one of these, it starts
with low strategic value.

| Workflow | Why it matters | Primary fixture / runner | Bias to watch |
|---------|----------------|--------------------------|---------------|
| Real demo-screen redraw loop | Closest in-tree proxy for production UI rendering across many widget mixes. | `profile_sweep --render-mode pipeline`, `demo_pipeline_bench` | Average frame time can hide heavy-screen tails; watch p95/p99 and emitted bytes. |
| Sparse-change render path | Determines whether dirty-region certificates and diff tuning are worth pursuing. | `ftui-render/benches/diff_bench.rs`, `profile_sweep` changed-cell counters | Mean latency can hide mode changes between sparse and bursty updates. |
| Full-screen or heavy-change redraws | Bounds worst-case presenter + write behavior and tests style-state churn. | `presenter_bench`, `profile_sweep` max bytes / changed cells | Tail spikes matter more than averages. |
| Pane drag / resize / replay loops | High-frequency interactive path with real branch pressure and tree churn. | `scripts/pane_profile.sh`, pane benches + perf-stat captures | Microbench speedups that do not improve replay-heavy scenarios are low EV. |
| Runtime subscription churn + shutdown | Critical for structured concurrency changes and operator-visible responsiveness. | runtime tests / evidence hooks in `Program`, later `rch`-offloaded targeted benches | Correctness and bounded shutdown dominate raw throughput. |
| doctor happy / failure / replay / determinism workflows | Operator-facing path where evidence quality is part of performance value. | doctor E2E scripts and verification report artifacts | Faster runs are worthless if manifests or replay bundles degrade. |

### 0.4 Metric Classification Rules

Later beads should classify metrics before collecting them:

| Metric family | Examples | Use when | Decision bias |
|--------------|----------|----------|---------------|
| Stage-local latency | diff time, presenter time, terminal write time, subscription stop latency | Choosing which stage to optimize | Prefer p95/p99 over mean for contention-prone paths |
| End-to-end latency | frame time, doctor command completion, pane gesture lifecycle time | Judging user/operator experience | Tail-sensitive |
| Output-cost metrics | changed cells, dirty spans, emitted bytes, stdout/stderr sizes, artifact counts | Distinguishing "less work" from "same work but faster CPU" | Often explanatory, not sufficient alone |
| Memory / allocation metrics | allocations/frame, allocated bytes/frame, replay retention size | Finding churn and retention regressions | Tail and max values matter |
| Throughput metrics | renders/sec, scenarios/min, replay throughput | Comparing steady-state efficiency | Only meaningful with environment fingerprinting |
| Integrity metrics | artifact completeness, replay success, divergence rate, schema validation pass/fail | Protecting diagnosability | Binary gate, not an optimization knob |

### 0.5 Required Join Keys For Performance Evidence

Any later baseline or hotspot artifact should be joinable across runtime, render,
doctor, and CI layers using at least:

- `bead_id` or an explicit analysis lane identifier
- `run_id`
- workload / fixture identifier
- terminal or viewport geometry when relevant
- scenario class (`render`, `runtime`, `pane`, `doctor`, `e2e`)
- artifact path or manifest reference

For existing tooling this usually means:

- `profile_sweep --json` output plus command line / geometry,
- Criterion benchmark name + input key,
- `target/pane-profiling/.../manifest.json` and `symbol_metadata.txt`,
- doctor `meta/summary.json` and `artifact_manifest.json`,
- suite-level `summary.json` from E2E scripts.

## 1. Pipeline Baselines

### 1.1 Text Shaping Pipeline (ftui-text)

| Operation | Input | Latency | Throughput | Notes |
|-----------|-------|---------|------------|-------|
| `ClusterMap::from_text` | latin/10K | 480µs | 19.8 MiB/s | Grapheme iteration + entry allocation |
| `ClusterMap::from_text` | cjk/10K | 343µs | 27.8 MiB/s | Fewer graphemes per byte |
| `ClusterMap::from_text` | latin/100K | 3.85ms | 24.7 MiB/s | Linear scaling confirmed |
| `ClusterMap::byte_to_cell` | 270 lookups/10K | 7µs | — | Binary search, O(log n) per lookup |
| `ClusterMap::cell_to_byte` | 270 lookups/10K | 8.4µs | — | Reverse binary search |
| `ClusterMap::cell_range_to_byte_range` | 200 lookups/10K | 17.6µs | — | Two binary searches per call |
| `ClusterMap::extract_text` | small ranges | 15.7µs | — | Multiple small extractions |
| `ShapedLineLayout::from_text` | latin/10K | 609µs | 15.6 MiB/s | Creates ClusterMap + placements |
| `ShapedLineLayout::from_text` | cjk/10K | 462µs | 20.6 MiB/s | |
| `ShapedLineLayout::from_run` | latin/10K | **53ms** | 183 KiB/s | **O(n^2) — CRITICAL HOTSPOT** |
| `ShapedLineLayout::from_run` | cjk/10K | 7.2ms | 1.3 MiB/s | 7x faster (fewer glyphs/byte) |
| `apply_justification` | 10K | 83µs | 115 MiB/s | Linear scan + spacing adjustment |
| `apply_tracking` | 10K | 47µs | 201 MiB/s | Simple per-placement addition |
| `placement_at_cell` | 270 lookups/10K | 905µs | — | Linear scan (not indexed) |

### 1.2 Width Calculation (ftui-text)

| Operation | Input | Latency | Throughput | Notes |
|-----------|-------|---------|------------|-------|
| `width` (ascii) | 1K chars | 6.5µs | 140 MiB/s | Fast path |
| `width` (cjk) | 1K chars | 33µs | 27 MiB/s | Unicode width lookup per char |
| `width` (emoji) | 1K chars | 56µs | 68 MiB/s | ZWJ/combining complexity |
| `segment_width` (ascii) | single | 65ns | — | Very fast |
| `segment_width` (cjk) | single | 6.4µs | — | |
| Cache warm hit | single | 1.4µs | — | ~2x over direct on repeated calls |

### 1.3 Width Cache (ftui-text)

| Operation | Pattern | Input | Latency | Notes |
|-----------|---------|-------|---------|-------|
| S3FIFO | zipfian | 10K | 409µs | Hot keys dominate |
| S3FIFO | scan | 10K | 1.1ms | Cold path, many misses |
| S3FIFO | mixed | 10K | 727µs | Real-world blend |
| S3FIFO | zipfian | 100K | 11.8ms | Linear scaling |
| S3FIFO | scan | 100K | 54.7ms | Cache thrashing |

### 1.4 Layout Solver (ftui-layout)

| Operation | Input | Latency | Notes |
|-----------|-------|---------|-------|
| Flex horizontal 3-child | simple | 44ns | Near-instant |
| Flex horizontal 10-child | constraints | 152ns | Linear in children |
| Flex horizontal 50-child | constraints | 544ns | |
| Flex vertical 20-child | split | 195ns | |
| Flex vertical 50-child | split | 374ns | |
| Grid 3x3 | split | 120ns | |
| Grid 10x10 | split | 390ns | |
| Grid 20x20 | split | 669ns | |
| Nested 3col x 10row | split | 337ns | Recursion overhead minimal |

### 1.4.1 Pane Workspace Baseline (bd-2bav7)

These SLA gates are enforced by `scripts/bench_budget.sh` using Criterion output
from `ftui-layout/layout_bench` and `ftui-web/pane_pointer_bench`.

| Benchmark key | Budget (ns) | Surface |
|-----------|---------|---------|
| `pane/core/solve_layout/leaf_count_8` | 200000 | Pane tree solve (small) |
| `pane/core/solve_layout/leaf_count_32` | 700000 | Pane tree solve (medium) |
| `pane/core/solve_layout/leaf_count_64` | 1400000 | Pane tree solve (large) |
| `pane/core/apply_operation/split_leaf` | 450000 | Structural split operation |
| `pane/core/apply_operation/move_subtree` | 900000 | Structural move operation |
| `pane/core/planning/plan_reflow_move` | 450000 | Reflow move planner |
| `pane/core/planning/plan_edge_resize` | 350000 | Edge resize planner |
| `pane/core/timeline/apply_and_replay_32_ops` | 2500000 | Timeline replay path |
| `pane/web_pointer/lifecycle/down_ack_move_32_up` | 1000000 | Host pointer lifecycle |
| `pane/web_pointer/lifecycle/down_ack_move_120_up` | 3500000 | Host pointer stress lifecycle |
| `pane/web_pointer/lifecycle/blur_after_ack` | 250000 | Host interruption path |

### 1.5 Render Pipeline (ftui-render)

| Operation | Input | Latency | Throughput | Notes |
|-----------|-------|---------|------------|-------|
| Cell `as_char` | single | 2.8ns | — | |
| PackedRGBA create | single | 0.9ns | — | |
| PackedRGBA `over` (partial) | single | 23.5ns | — | Alpha blending |
| Row compare (identical) | 80 cells | 155ns | — | SIMD-friendly |
| Row compare (identical) | 200 cells | 443ns | — | |
| BufferDiff compute | 240x80 sparse 5% | 28µs | 686 Melem/s | |
| BufferDiff compute | 240x80 single row | 19µs | 1.0 Gelem/s | |
| BufferDiff compute_dirty | 240x80 single row | 16µs | 1.17 Gelem/s | |
| Presenter (sparse 5%) | 80x24 | 5.3µs | 7.2 Melem/s | |
| Presenter (sparse 5%) | 200x60 | 63µs | 9.6 Melem/s | |
| Presenter (heavy 50%) | 200x60 | 113µs | 5.7 Melem/s | |
| Presenter (full 100%) | 200x60 | 370µs | 5.1 Melem/s | |
| Full pipeline (diff+present) | 80x24@5% | 18.6µs | 103 Melem/s | |
| Full pipeline (diff+present) | 200x60@5% | 71.7µs | 167 Melem/s | |
| Full pipeline (diff+present) | 200x60@50% | 89µs | 135 Melem/s | |

### 1.6 Shaping Fallback Pipeline (ftui-text)

| Operation | Input | Latency | Throughput | Notes |
|-----------|-------|---------|------------|-------|
| Terminal mode | latin/10K | 612µs | 15.5 MiB/s | from_text path |
| Terminal mode | cjk/10K | 498µs | 19.1 MiB/s | |
| Shaped NoopShaper | latin/10K | **60.8ms** | 160 KiB/s | **from_run O(n^2) dominates** |
| Shaped NoopShaper | mixed/10K | 29ms | 331 KiB/s | |
| Batch terminal | 40 lines | 209µs | 15.2 MiB/s | Per-screenful budget |

## 2. Hotspot Opportunity Matrix

Ranked by impact (latency × frequency) and optimization confidence:

| Rank | Hotspot | Current | Target | Speedup | Effort | Confidence | Blocks |
|------|---------|---------|--------|---------|--------|------------|--------|
| **1** | `ShapedLineLayout::from_run` O(n^2) | 53ms/10K | <1ms/10K | **50-100x** | Medium | High | Shaped path unusable for real text |
| **2** | `placement_at_cell` linear scan | 905µs/270 lookups | <50µs | **~18x** | Low | High | Add cell-index array or binary search |
| **3** | `ClusterMap::from_text` allocation | 480µs/10K | ~200µs | **~2x** | Medium | Medium | Pre-allocate Vec with capacity hint |
| **4** | Width cache scan pattern | 54.7ms/100K | ~12ms | **~4x** | Medium | Medium | S3FIFO eviction tuning or CLOCK-Pro |
| **5** | Presenter full-screen | 370µs/200x60 | ~200µs | **~1.8x** | High | Low | Already state-tracked; diminishing returns |
| **6** | `from_text` layout construction | 609µs/10K | ~400µs | **~1.5x** | Medium | Medium | Reduce ClusterMap + placement allocation |

### Scoring Key

- **Effort**: Low = <2h, Medium = 2-8h, High = 8h+
- **Confidence**: High = clear algorithmic fix, Medium = needs profiling, Low = near-optimal already
- **Speedup**: Estimated improvement factor

## 3. Critical Path for 60fps Budget

At 60fps, frame budget = 16.67ms. Key pipeline stages:

| Stage | Budget Share | Current (200x60) | Status |
|-------|-------------|-------------------|--------|
| Layout solve | 5% (0.8ms) | ~1µs | Well within budget |
| Text shaping (terminal) | 15% (2.5ms) | ~612µs/10K | OK for screen-sized text |
| Text shaping (shaped) | 15% (2.5ms) | **53ms** | **BLOCKS 60fps** |
| Buffer diff | 10% (1.7ms) | ~28µs | Well within budget |
| Presenter (ANSI emit) | 20% (3.3ms) | ~370µs (full) | OK |
| Headroom | 50% | — | Available for widgets, IO |

## 4. Recommendations

1. **Fix `from_run` O(n^2)** before enabling shaped rendering in production. The `sum_cluster_advance` and/or `render_hint_for_cluster` helpers likely do linear scans per glyph. Switch to pre-computed cluster boundaries.

2. **Index `placement_at_cell`** with a cell-offset lookup table for O(1) access instead of linear scan.

3. **Profile `ClusterMap::from_text`** with flame graphs to identify whether grapheme iteration or Vec allocation dominates.

4. **Terminal fallback path is production-ready** at 612µs/10K (~15 MiB/s). No optimization needed for typical terminal workloads.

5. **Layout solver is extremely fast** (<1µs for typical layouts). No optimization needed.

6. **Diff + Present pipeline** is well-optimized at ~90µs for 200x60@50% change. No immediate action.

## 5. Repro Commands (Pane SLA)

Use `rch` for CPU-heavy benchmark commands:

```bash
mkdir -p target/benchmark-results
rch exec -- cargo bench -p ftui-layout --bench layout_bench -- pane/core/ \
  | tee target/benchmark-results/layout_bench.txt
rch exec -- cargo bench -p ftui-layout --bench pane_profile_harness -- \
  --out-dir target/pane-profiling/manual/pane_core_profile_harness \
  --iterations 2000 --warmup-iterations 200 \
  | tee target/benchmark-results/pane_core_profile_harness.txt
rch exec -- cargo bench -p ftui-runtime --bench pane_terminal_bench -- pane/terminal/ \
  | tee target/benchmark-results/pane_terminal_bench.txt
rch exec -- cargo bench -p ftui-web --bench pane_pointer_bench -- pane/web_pointer/ \
  | tee target/benchmark-results/pane_pointer_bench.txt
./scripts/bench_budget.sh --check-only
./scripts/bench_budget.sh --json
./scripts/pane_profile.sh --test
./scripts/pane_profile.sh --test --time
./scripts/pane_profile.sh --test --perf-stat
```

Budget logs are emitted to:

- `target/benchmark-results/perf_log.jsonl`
- `target/benchmark-results/perf_confidence.jsonl`

The long-lived pane-core harness also emits:

- `target/pane-profiling/.../pane_core_profile_harness/manifest.json`
- `target/pane-profiling/.../pane_core_profile_harness/baseline_snapshot.json`
- `target/pane-profiling/.../pane_core_profile_harness/final_snapshot.json`
- `target/pane-profiling/.../pane_core_profile_harness/run.log`

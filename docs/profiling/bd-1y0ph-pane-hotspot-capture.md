# bd-1y0ph Pane Hotspot Capture

Purpose
- Establish a repeatable profiling checkpoint for pane performance across the
  core solver/operation path, terminal drag adapter path, and web pointer
  adapter path.

Status
- `bd-1y0ph` is closed as of 2026-04-25 after exact-binary provenance was
  verified across all captured pane profiling surfaces.
- The repeatable capture runner is `./scripts/pane_profile.sh`.
- Current artifact directory: `target/pane-profiling/bd-1y0ph/`
- Latest exact-binary trust verification:
  `target/pane-profiling/bd-1y0ph-windybeaver/`

## Captured Surfaces

The current test-mode capture covers:

- `pane/core/*` via `ftui-layout/layout_bench`
- `pane/terminal/*` via `ftui-runtime/pane_terminal_bench`
- `pane/web_pointer/*` via `ftui-web/pane_pointer_bench`

Artifacts produced by the latest run:

- `target/pane-profiling/bd-1y0ph/pane_core_profile_harness.txt`
- `target/pane-profiling/bd-1y0ph/pane_core_profile_harness/manifest.json`
- `target/pane-profiling/bd-1y0ph/pane_core_profile_harness/baseline_snapshot.json`
- `target/pane-profiling/bd-1y0ph/pane_core_profile_harness/final_snapshot.json`
- `target/pane-profiling/bd-1y0ph/pane_core_profile_harness/run.log`
- `target/pane-profiling/bd-1y0ph/layout_bench.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_bench.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_bench.txt`
- `target/pane-profiling/bd-1y0ph/layout_bench.perfstat.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_bench.perfstat.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_bench.perfstat.txt`
- `target/pane-profiling/bd-1y0ph/pane_core_timeline_apply_and_replay_32_ops.perf.data`
- `target/pane-profiling/bd-1y0ph/pane_core_timeline_apply_and_replay_32_ops.perf.txt`
- `target/pane-profiling/bd-1y0ph/pane_core_timeline_apply_and_replay_32_ops.symbols.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_down_drag_120_up.perf.data`
- `target/pane-profiling/bd-1y0ph/pane_terminal_down_drag_120_up.perf.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_down_drag_120_up.symbols.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_down_ack_move_120_up.perf.data`
- `target/pane-profiling/bd-1y0ph/pane_pointer_down_ack_move_120_up.perf.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_down_ack_move_120_up.symbols.txt`
- `target/pane-profiling/bd-1y0ph/executed-binaries/`
- `target/pane-profiling/bd-1y0ph/symbol_metadata.txt`

All targeted probes passed in the latest test-mode capture.

## Repro

Use `rch` indirectly through the runner:

```bash
./scripts/pane_profile.sh --test
./scripts/pane_profile.sh --test --time
./scripts/pane_profile.sh --test --perf-stat
./scripts/pane_profile.sh --test --stack-reports
```

For later non-test captures:

```bash
./scripts/pane_profile.sh
```

## First-Principles Hotspot Hypotheses

These are code-path hypotheses from the current implementation, now backed by
`perf stat` counter samples for one representative benchmark per surface.

### Core pane path (`ftui-layout/src/pane.rs`)

- `PaneTree::solve_layout()` is recursive and writes every visited node into a
  `BTreeMap`, so deep trees pay repeated map insert and lookup costs.
- `PaneTree::apply_operation()` clones the full tree before every structural
  operation, which is a likely allocation and copy hotspot for drag-heavy edit
  loops.
- `PaneInteractionTimeline::replay()` rebuilds from baseline and reapplies all
  retained operations, so replay cost grows with timeline length.

Representative `perf stat` sample (`pane/core/timeline/apply_and_replay_32_ops`):

- ~408M instructions / ~157M cycles over ~44.5ms elapsed
- 2.60 IPC, ~1.22% branch-miss rate
- ~3.05% L1 d-cache load miss rate

Interpretation:

- This path is compute-dense rather than branch- or cache-pathological.
- The first optimization wins are still algorithmic: less replay work and fewer
  tree clones, not micro-tuning branches.

### Terminal pane path (`ftui-runtime/src/program.rs`)

- `PaneTerminalAdapter::translate_with_handles()` performs handle resolution on
  every relevant mouse event before semantic translation.
- Drag updates compute cumulative motion metadata and call
  `Instant::elapsed()` during translation, so the drag loop includes time-query
  overhead in addition to state-machine transitions.
- The semantic forwarding path constructs validated pane events for each input,
  which is likely cheaper than the core structural path but still a meaningful
  per-event cost center.

Representative `perf stat` sample (`pane/terminal/lifecycle/down_drag_120_up`):

- ~6.7M instructions / ~7.5M cycles over ~3.59ms elapsed
- 0.89 IPC, ~4.03% branch-miss rate
- ~1.13% L1 d-cache load miss rate

Interpretation:

- This path is branchier and less pipeline-efficient than the core replay path.
- The likely cost centers are event-state branching and handle-resolution logic,
  not cache misses.

### Web pane path (`ftui-web/src/pane_pointer_capture.rs`)

- `PanePointerCaptureAdapter::pointer_move()` maintains cumulative motion and
  direction-change tracking on every move event.
- The web adapter avoids terminal hit-resolution overhead, so the expected
  comparison point is whether state-machine forwarding or motion bookkeeping
  dominates after capture is acquired.

Representative `perf stat` sample (`pane/web_pointer/lifecycle/down_ack_move_120_up`):

- ~2.7M instructions / ~4.1M cycles over ~2.33ms elapsed
- 0.65 IPC, ~8.38% branch-miss rate
- ~3.16% L1 d-cache load miss rate

Interpretation:

- This is the branchiest of the three sampled surfaces.
- The web pointer path likely pays for high-frequency conditional state updates
  and direction-change bookkeeping more than raw arithmetic.

## Environment Notes

`perf` is enabled in the current environment after lowering
`kernel.perf_event_paranoid` to `1`, so counter-based profiling is now part of
the repeatable capture workflow.

The profiling runner now emits `symbol_metadata.txt`, records the exact bench
binary path printed by the current run, and preserves a copy of the exact
executed bench binary under `executed-binaries/` for every surface. When `rch`
ran the benchmark on a worker, the runner first uses the retrieved local binary
when present and falls back to explicit remote fetch if needed.

Important `rch` trust boundary: remote fetches must use the selected worker
alias, such as `ts2`, not the printed endpoint, such as
`ubuntu@192.168.1.124`. The alias carries the SSH identity and host options
that make the worker reachable. The endpoint is recorded separately as
`worker_endpoint` for audit context only.

For the pane-core lane, the runner also invokes a long-lived deterministic
profile harness that emits:

- a machine-readable `manifest.json`
- golden `baseline_snapshot.json` and `final_snapshot.json`
- verbose `run.log`

The harness manifest and log now carry checkpoint-specific diagnostics as
first-class evidence:

- `checkpoint_interval`
- `checkpoint_count`
- `checkpoint_hit`
- `replay_start_idx`
- `replay_depth`
- `estimated_snapshot_cost_ns`
- `estimated_replay_step_cost_ns`
- `checkpoint_decision`

That harness exists so later checkpointing and memory work can reuse one
replay-heavy scenario with stable hashes instead of reconstructing the same
timeline workload from ad hoc bench flags.

The general-purpose Criterion pane timeline bench in `ftui-layout/layout_bench`
also now exposes short, medium, and long replay histories under stable names:

- `pane/core/timeline/apply_and_replay_8_ops`
- `pane/core/timeline/apply_and_replay_32_ops`
- `pane/core/timeline/apply_and_replay_256_ops`

The artifact contract is now explicit per surface:

- `executed_path`: the bench binary path reported by the current run
- `worker`: the `rch` worker used for the run, or `local`
- `worker_endpoint`: the raw endpoint printed by `rch`, or `local`
- `binary_source`: whether the exact binary is local, remotely fetched, or
  preserved from the retrieved local path, or still unavailable
- `exact_binary_status`: `available` or `missing`
- `exact_binary_local`: local path to the exact binary artifact when present
- `fetch_error`: machine-checkable reason when remote materialization fails
- `local_candidate`: an informational local candidate path only; it is no
  longer treated as authoritative exact-binary evidence

Current verified evidence is now clean across all profiled surfaces:

- `pane_profile_harness`: exact binary preserved under `executed-binaries/`
  with debug info
- `layout_bench`: exact binary preserved under `executed-binaries/` with
  debug info
- `pane_terminal_bench`: exact binary preserved under `executed-binaries/`
  with debug info
- `pane_pointer_bench`: exact binary preserved under `executed-binaries/`
  with debug info

That means symbol-readiness is now explicit and durable: the artifact bundle
contains both the provenance metadata and preserved exact binaries instead of
silently relying on whichever file happens to still be present under
`target/release/deps`.

Latest verified `symbol_metadata.txt` state from
`./scripts/pane_profile.sh --test --out-dir target/pane-profiling/bd-1y0ph-windybeaver`
on 2026-04-25:

```text
pane_profile_harness: worker=ts2, worker_endpoint=ubuntu@192.168.1.124, binary_source=executed_remote_fetched, exact_binary_status=available, debug_info=present
layout_bench: worker=ts2, worker_endpoint=ubuntu@192.168.1.124, binary_source=executed_remote_fetched, exact_binary_status=available, debug_info=present
pane_terminal_bench: worker=ts2, worker_endpoint=ubuntu@192.168.1.124, binary_source=executed_remote_fetched, exact_binary_status=available, debug_info=present
pane_pointer_bench: worker=ts2, worker_endpoint=ubuntu@192.168.1.124, binary_source=executed_remote_fetched, exact_binary_status=available, debug_info=present
```

## Representative Stack Evidence

The runner now emits post-symbolized user-space top-frame summaries for the three
representative profiling targets. These are not full flamegraphs, but they are
good enough to rank optimization lanes without reconstructing symbol trust by
hand.

Representative artifacts:

- `target/pane-profiling/bd-1y0ph/pane_core_timeline_apply_and_replay_32_ops.symbols.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_down_drag_120_up.symbols.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_down_ack_move_120_up.symbols.txt`

Current top user-space findings:

- Core replay sample is dominated by allocation-heavy hash table and layout
  builder paths such as `hashbrown::raw::RawTable::reserve_rehash`,
  `ftui_layout::veb_tree::VebTree::build`, and
  `ftui_layout::veb_tree::veb_layout_order`.
- Terminal drag sample currently collapses into pane-core validation work,
  especially `PaneTree::validate` and BTree search in
  `alloc::collections::btree::node::NodeRef::search_tree`.
- Web pointer sample is concentrated in
  `PanePointerCaptureAdapter::pointer_up`, which means the expensive part of
  the lifecycle is not raw move arithmetic alone but the terminal state update
  and commit path reached at gesture completion.

Trust notes:

- User-space symbol trust is now `high` for all three surfaces because the
  exact executed binaries are preserved locally and the post-symbolized
  summaries are derived from perf mmap metadata plus those preserved binaries.
- Kernel-side frames remain partially unresolved because `kptr_restrict`
  prevents full kernel symbol expansion in this environment. That does not
  block the pane-specific ranking because the actionable work is in the
  user-space frames above.

Residual uncertainty:

- The core stack sample comes from the `layout_bench` representative path and
  therefore still mixes replay work with supporting data-structure costs inside
  the same exact benchmark binary.
- The terminal and web samples are representative lifecycle captures, so they
  say which code paths dominate the benchmarked interaction, not necessarily
  which individual event step is worst in isolation.

## Final Opportunity Matrix

Ranked for downstream optimization work using impact, confidence, effort, and
 proof-readiness from the current artifact bundle.

| Rank | Lane | Dominant evidence | Why it matters | Impact | Confidence | Effort | Proof-ready next bead(s) |
|------|------|-------------------|----------------|--------|------------|--------|--------------------------|
| 1 | Pane core structural validation + replay internals | `pane_core_timeline_apply_and_replay_32_ops.symbols.txt`, `pane_core_profile_harness/*`, `layout_bench.perfstat.txt` | Allocation-heavy table growth, VEB/layout-order work, and validation/search traffic dominate the replay-heavy surface and also show up under terminal drag. | High | High | Medium-High | `bd-1k7ek.2`, `bd-1k7ek.3` |
| 2 | Web pointer commit / gesture-finalization path | `pane_pointer_down_ack_move_120_up.symbols.txt`, `pane_pointer_bench.perfstat.txt` | The representative web lifecycle is concentrated in `pointer_up`, so trimming bookkeeping and commit-path branching should improve perceived drag completion and reduce branch-heavy tail behavior. | Medium-High | Medium | Medium | `bd-1k7ek.8` |
| 3 | Terminal drag translation and handle-routing path | `pane_terminal_down_drag_120_up.symbols.txt`, `pane_terminal_bench.perfstat.txt` | Terminal drag is still expensive, but the trustworthy stack sample says much of that cost currently lands in pane-core validation rather than adapter logic alone. Optimize adapter branching after reducing core validation churn. | Medium | Medium | Medium | `bd-1k7ek.7` |

Optimization order justified by the current evidence:

1. Reduce pane-core replay/validation churn first, because that is both a
   first-order core bottleneck and a second-order contributor to terminal drag.
2. Simplify the web pointer commit path next, because its stack sample is
   already sharply localized and has a good chance of yielding visible wins
   without broad architectural change.
3. Revisit terminal adapter state-machine cleanup after the core fast paths land
   so we do not mistake downstream validation cost for front-end routing cost.

## Handoff Summary

This bead now has a self-contained artifact chain:

- exact executed binaries with explicit trust metadata
- representative counter summaries per pane surface
- representative stack reports and post-symbolized user-space summaries
- a final opportunity matrix with residual uncertainty notes

That is enough to justify the downstream optimization order without making
future sessions reconstruct the evidence chain from raw `perf` output.

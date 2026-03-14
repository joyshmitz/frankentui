# bd-1y0ph Pane Hotspot Capture

Purpose
- Establish a repeatable profiling checkpoint for pane performance across the
  core solver/operation path, terminal drag adapter path, and web pointer
  adapter path.

Status
- `bd-1y0ph` remains `in_progress`.
- The repeatable capture runner is `./scripts/pane_profile.sh`.
- Current artifact directory: `target/pane-profiling/bd-1y0ph/`

## Captured Surfaces

The current test-mode capture covers:

- `pane/core/*` via `ftui-layout/layout_bench`
- `pane/terminal/*` via `ftui-runtime/pane_terminal_bench`
- `pane/web_pointer/*` via `ftui-web/pane_pointer_bench`

Artifacts produced by the latest run:

- `target/pane-profiling/bd-1y0ph/layout_bench.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_bench.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_bench.txt`
- `target/pane-profiling/bd-1y0ph/layout_bench.perfstat.txt`
- `target/pane-profiling/bd-1y0ph/pane_terminal_bench.perfstat.txt`
- `target/pane-profiling/bd-1y0ph/pane_pointer_bench.perfstat.txt`
- `target/pane-profiling/bd-1y0ph/symbol_metadata.txt`

All targeted probes passed in the latest test-mode capture.

## Repro

Use `rch` indirectly through the runner:

```bash
./scripts/pane_profile.sh --test
./scripts/pane_profile.sh --test --time
./scripts/pane_profile.sh --test --perf-stat
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

The profiling runner now emits `symbol_metadata.txt`, and it records the exact
bench binary path printed by the current run instead of guessing from whatever
matching binary happens to exist under `target/release/deps/`.

Current evidence from that artifact is mixed:

- `layout_bench`: `with debug_info, not stripped`
- `pane_terminal_bench`: exact executed binary path captured, but local binary
  currently missing after `rch` artifact retrieval
- `pane_pointer_bench`: exact executed binary path captured, but local binary
  currently missing after `rch` artifact retrieval

That means symbol-readiness is no longer an assumption, but it also means the
terminal/web benches still need follow-up if we want uniformly trustworthy
`perf report` stack attribution across all pane surfaces from the local artifact
bundle. The immediate value of the new metadata is that it distinguishes
"missing exact executed binary" from "found a different local binary and guessed."

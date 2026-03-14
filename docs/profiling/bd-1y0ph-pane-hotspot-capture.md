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

All targeted probes passed in the latest test-mode capture.

## Repro

Use `rch` indirectly through the runner:

```bash
./scripts/pane_profile.sh --test
```

For later non-test captures:

```bash
./scripts/pane_profile.sh
```

## First-Principles Hotspot Hypotheses

These are code-path hypotheses from the current implementation, not flamegraph
proof yet.

### Core pane path (`ftui-layout/src/pane.rs`)

- `PaneTree::solve_layout()` is recursive and writes every visited node into a
  `BTreeMap`, so deep trees pay repeated map insert and lookup costs.
- `PaneTree::apply_operation()` clones the full tree before every structural
  operation, which is a likely allocation and copy hotspot for drag-heavy edit
  loops.
- `PaneInteractionTimeline::replay()` rebuilds from baseline and reapplies all
  retained operations, so replay cost grows with timeline length.

### Terminal pane path (`ftui-runtime/src/program.rs`)

- `PaneTerminalAdapter::translate_with_handles()` performs handle resolution on
  every relevant mouse event before semantic translation.
- Drag updates compute cumulative motion metadata and call
  `Instant::elapsed()` during translation, so the drag loop includes time-query
  overhead in addition to state-machine transitions.
- The semantic forwarding path constructs validated pane events for each input,
  which is likely cheaper than the core structural path but still a meaningful
  per-event cost center.

### Web pane path (`ftui-web/src/pane_pointer_capture.rs`)

- `PanePointerCaptureAdapter::pointer_move()` maintains cumulative motion and
  direction-change tracking on every move event.
- The web adapter avoids terminal hit-resolution overhead, so the expected
  comparison point is whether state-machine forwarding or motion bookkeeping
  dominates after capture is acquired.

## Next Evidence Step

The next `bd-1y0ph` slice should add CPU/allocation profile captures for the
highest-frequency terminal and core paths, using the new runner artifacts as the
stable benchmark/probe baseline.

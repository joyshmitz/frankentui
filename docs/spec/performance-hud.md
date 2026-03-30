# Performance HUD — Spec + UX Flow

This spec defines the user experience, behavior, and test requirements for the
**Performance HUD + Render Budget Visualizer** (`bd-3k3x.1`). The HUD is a
lightweight, always-on-demand overlay that surfaces frame-budget health and
render pipeline stats in real time.

User guide: `docs/performance-hud.md`  
Demo script: `scripts/perf_hud_demo.sh`

---

## 1) Goals
- Provide a **low-overhead**, always-available view of render health.
- Make frame budget usage and degradation **explicit** and **actionable**.
- Use deterministic formatting and stable ordering to avoid visual flicker.
- Work in **inline** and **alt-screen** modes without breaking the one-writer rule.

## 2) Non-Goals
- Full profiler UI (no flamegraphs, no allocations listing).
- Per-widget tracing (handled elsewhere if needed).
- Remote telemetry storage (HUD is local display only).

---

## 3) UX States

### 3.1 Hidden (Default)
- No HUD displayed.
- No cost beyond cheap counters already collected.

### 3.2 Overlay (Active)
- HUD overlays the UI in a fixed corner (default: top-right).
- Uses a compact panel with stable rows for metrics.
- Must not consume input focus; keyboard interaction remains with the app.

### 3.3 Degraded / Minimal
- If terminal is too small, show a single-line summary:
  - `HUD: 12.4ms / 16ms | Δ 512 cells | 9.6KB | FULL`

---

## 4) Input Model & Keybindings

- **Toggle HUD:** `Ctrl+P` (current demo binding)
- **Cycle detail level:** `H` (Hidden → Compact → Full → Hidden)
- **Freeze HUD snapshot:** `f` (optional; stops updating until toggled)

Help overlay must list these bindings when HUD is enabled.

---

## 5) Metrics (Required)

### 5.1 Frame Budget
Derived from `RenderBudget`:
- `total_ms`
- `elapsed_ms`
- `remaining_ms`
- `degradation_level` (Full, Reduced, Minimal, Skeleton, Off)
- `frame_skip_allowed` (boolean)

### 5.2 Render / Present
Derived from `PresentStats`:
- `cells_changed`
- `run_count`
- `bytes_emitted`
- `present_duration_ms`
- `bytes_per_cell` and `bytes_per_run`

### 5.3 Runtime Loop
- `tick_count`
- `input_backlog_len` (if the runtime exposes one separately from the effect queue)
- `last_frame_ms` (end-to-end frame time)
- `dropped_frames` (if frame skipping is enabled)

### 5.4 Runtime Pressure + Control Plane
Derived from the runtime-performance control contract:
- `runtime_mode` (`healthy`, `stressed`, `degraded`, `recovered`)
- `pressure_class`
- `work_disposition` summary (canonical values: `preserved`, `coalesced`, `deferred`, `dropped_with_reason`, `failed_fast`)
- effect-queue depth (`in_flight`), `capacity`, `high_water`, `dropped_count`
- `coalescing_state`, `coalesced_count`, `deferred_count`
- resize `regime` and whether a hard-deadline apply was recently forced

### 5.5 Capabilities (Compact)
- `sync_output` (DEC 2026)
- `scroll_region` support
- `osc8` hyperlinks

---

## 6) Layout & Visual Design

### 6.1 Compact Layout (default)
```
┌─ Performance HUD ─────────────┐
│ Frame:   11.8 / 16.0 ms       │
│ Budget:  4.2 ms remaining     │
│ Δ Cells: 512  Runs: 18        │
│ Bytes:   9.6 KB  B/Cell: 19   │
│ Deg:     FULL   Drops: 0      │
└───────────────────────────────┘
```

### 6.2 Full Layout
- Adds runtime loop stats, capability flags, and sampling window statistics.
- Adds runtime mode, pressure class, queue occupancy/capacity, active
  coalescing state, deferred/coalesced/dropped counts, and work-disposition
  rows.
- Adds a mini “budget bar” (filled proportionally to elapsed/total).

### 6.3 Minimal Layout
- Single-line summary when space constrained.
- No borders or padding; use muted style.

---

## 7) Determinism + Invariants

- HUD output is deterministic given identical inputs.
- No per-frame heap allocations in HUD rendering (pre-allocated buffers only).
- Row order and formatting are stable across frames.
- If a metric is unavailable, show `n/a` (never panic).

---

## 8) Failure Modes (Ledger)

- **Missing stats:** Display `n/a`, do not crash.
- **Budget overrun:** Show `OVER` state; trigger visible warning style.
- **Tiny terminal:** Fallback to minimal single-line summary.
- **Frame skip on:** Explicitly show `Drops` counter.

Evidence ledger fields:
- `hud_level` (hidden/compact/full/minimal)
- `reason_code` (`terminal_size`, `budget_overrun`, `no_stats`)
- `degradation_level`
- `runtime_mode`
- `pressure_class`
- `coalescing_state`
- `coalesced_count`
- `deferred_count`
- `work_disposition`
- `queue_depth`
- `queue_capacity`
- `queue_high_water`
- `dropped_count`
- `elapsed_ms`, `total_ms`

---

## 9) Performance & Optimization Protocol

- Baseline render cost (p50/p95/p99) with `hyperfine` running demo showcase.
- Profile CPU + allocations to confirm HUD does not dominate frame time.
- Opportunity matrix (Impact×Confidence/Effort ≥ 2.0 to act).
- Isomorphism proof: metric ordering + formatting string are stable; golden
  output checksum for the same input snapshot.

---

## 10) Tests (Required)

### Unit Tests
- Formatting for compact/full/minimal layouts.
- Degradation display mapping from `DegradationLevel`.
- Deterministic output for identical input snapshot.

### Property Tests
- Idempotence: rendering HUD twice with same inputs yields same output.
- Ordering invariant: row order never changes across frames.

### Snapshot Tests
- Compact layout
- Full layout
- Minimal layout
- Over-budget warning state

### PTY E2E
- Toggle HUD on/off
- Resize to minimal terminal size
- Verify JSONL logs include:
  - `frame_idx`, `elapsed_ms`, `total_ms`
  - `cells_changed`, `run_count`, `bytes_emitted`
  - `degradation_level`, `drops`
  - `runtime_mode`, `pressure_class`, `coalescing_state`, `work_disposition`
  - `coalesced_count`, `deferred_count`, `dropped_count`
  - `queue_depth`, `queue_capacity`, `queue_high_water`
  - terminal `rows`, `cols`

---

## 11) Integration Notes

- HUD should live in the **Performance** screen first, then optionally in
  global chrome if the runtime provides hooks.
- Stats should flow from `RenderBudget` and `PresentStats` (no custom timers).
- The HUD must respect the one-writer rule and render via existing Frame APIs.

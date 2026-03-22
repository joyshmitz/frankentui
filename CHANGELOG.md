# Changelog

All notable changes to [FrankenTUI](https://github.com/Dicklesworthstone/frankentui) are documented here, organized by landed capabilities within each version.

**Repo:** <https://github.com/Dicklesworthstone/frankentui>
**Crate:** `ftui` (facade) plus 19 workspace crates
**License:** MIT + OpenAI/Anthropic Rider

---

## [Unreleased] (after v0.2.1)

> Commits on `main` since the v0.2.1 tag (2026-03-07).
> Compare: <https://github.com/Dicklesworthstone/frankentui/compare/v0.2.1...main>

### Accessibility (ftui-a11y)

- New `ftui-a11y` crate with full accessibility tree infrastructure (Phase 1, issue #44). ([b9952a01](https://github.com/Dicklesworthstone/frankentui/commit/b9952a01))
- `Accessible` trait implemented for 9 core widgets: Block, Paragraph, Scrollbar, Spinner, plus 5 initial widgets. ([756d42bc](https://github.com/Dicklesworthstone/frankentui/commit/756d42bc), [0d044784](https://github.com/Dicklesworthstone/frankentui/commit/0d044784))
- Accessibility proxy layer added to the demo showcase with ACCESSIBILITY.md documentation. ([b3f12877](https://github.com/Dicklesworthstone/frankentui/commit/b3f12877))
- Grapheme-aware title fitting, scissor-respecting styles, and soft-wrap page nav with a11y hardening. ([63188cb6](https://github.com/Dicklesworthstone/frankentui/commit/63188cb6))

### Render Pipeline Performance

- Hand-rolled byte-buffer ANSI emission replaces `format!`/`write!` in the presenter hot path. ([3e298b64](https://github.com/Dicklesworthstone/frankentui/commit/3e298b64))
- Span-aware tile diff builder skips clean rows; reconcile_subscriptions guarded behind `self.running`. ([870d24f1](https://github.com/Dicklesworthstone/frankentui/commit/870d24f1))
- Hyperlink bookkeeping skipped entirely when no links are active or registered. ([7a910893](https://github.com/Dicklesworthstone/frankentui/commit/7a910893))
- `PreparedContent` enum introduced to optimize presenter dispatch. ([59d7709e](https://github.com/Dicklesworthstone/frankentui/commit/59d7709e))
- Quad-cell diff skip, same-row CUP elimination, ASCII `PreparedContent` fast path, and cached hyperlink policy. ([10c341f5](https://github.com/Dicklesworthstone/frankentui/commit/10c341f5))
- `row_cells_mut_span()` added for bulk mutable row access with dirty tracking. ([be300dda](https://github.com/Dicklesworthstone/frankentui/commit/be300dda))
- Thread-local paragraph metrics and wrap caching for widgets. ([e1fb48d3](https://github.com/Dicklesworthstone/frankentui/commit/e1fb48d3))

### Render Gauntlet and Certificate-Based Diff Elision

- `DiffSkipHint` for certificate-based diff elision in the render pipeline. ([e99c4448](https://github.com/Dicklesworthstone/frankentui/commit/e99c4448))
- Render gauntlet framework with equivalence modules in the harness. ([c5332777](https://github.com/Dicklesworthstone/frankentui/commit/c5332777))
- Fixture runner and doctor cost profiling modules. ([ba46183d](https://github.com/Dicklesworthstone/frankentui/commit/ba46183d))
- Baseline capture and hotspot extraction modules. ([0eee1e19](https://github.com/Dicklesworthstone/frankentui/commit/0eee1e19))

### Asupersync Integration

- ADR-010 for targeted Asupersync adoption in runtime shell and doctor. ([dbfad8c1](https://github.com/Dicklesworthstone/frankentui/commit/dbfad8c1))
- Asupersync-executor backend for blocking task execution in the runtime. ([2c3d6b17](https://github.com/Dicklesworthstone/frankentui/commit/2c3d6b17))
- `TaskExecutor` seam extracted; doctor summary artifact persisted. ([6d0f1c2e](https://github.com/Dicklesworthstone/frankentui/commit/6d0f1c2e))
- Panic-resilient task executor with backpressure evidence. ([24ab6918](https://github.com/Dicklesworthstone/frankentui/commit/24ab6918))
- Seam inventory spec and invariant/metrics/evidence documentation. ([04a1c021](https://github.com/Dicklesworthstone/frankentui/commit/04a1c021), [2e40451d](https://github.com/Dicklesworthstone/frankentui/commit/2e40451d))

### Doctor Diagnostics Expansion

- Tmux observe mode for app smoke fallback. ([8462bb28](https://github.com/Dicklesworthstone/frankentui/commit/8462bb28))
- Enhanced diagnostic capture, reporting, and test suite. ([533a55ca](https://github.com/Dicklesworthstone/frankentui/commit/533a55ca))
- Trace, fallback, and capture error profile tracking in reports. ([ad189fd6](https://github.com/Dicklesworthstone/frankentui/commit/ad189fd6))
- Orchestration workflow documentation and profiling scripts. ([f9f1e4e2](https://github.com/Dicklesworthstone/frankentui/commit/f9f1e4e2))
- MCP tool-level `isError` response detection. ([54acfc59](https://github.com/Dicklesworthstone/frankentui/commit/54acfc59))

### Runtime and Harness

- Effect executor, lab harness, and subscription engine expansion (+1184 lines). ([4f3fe05f](https://github.com/Dicklesworthstone/frankentui/commit/4f3fe05f))
- Subscription engine and lifecycle contract tests (+993 lines). ([014ee3a9](https://github.com/Dicklesworthstone/frankentui/commit/014ee3a9))
- Rollout scorecard, seed RPC expansion, and program lifecycle improvements. ([816060bb](https://github.com/Dicklesworthstone/frankentui/commit/816060bb), [19d7f04b](https://github.com/Dicklesworthstone/frankentui/commit/19d7f04b))
- Rollout runbook module and failure E2E script (+475 lines). ([cd9005ab](https://github.com/Dicklesworthstone/frankentui/commit/cd9005ab))
- Artifact manifest, doctor topology, failure signatures, and proof oracle modules. ([2cfce9d4](https://github.com/Dicklesworthstone/frankentui/commit/2cfce9d4))
- Telemetry schema module. ([624db88c](https://github.com/Dicklesworthstone/frankentui/commit/624db88c))
- Go/no-go scorecard test for runtime readiness. ([9c378c17](https://github.com/Dicklesworthstone/frankentui/commit/9c378c17))

### Web and Layout

- Pane pointer capture state tracking corrected. ([b1ee6e2f](https://github.com/Dicklesworthstone/frankentui/commit/b1ee6e2f))
- Pane pointer benchmark and refined tests. ([3c48d347](https://github.com/Dicklesworthstone/frankentui/commit/3c48d347))
- Pane layout engine expansion with Asupersync seam inventory. ([3aa074ad](https://github.com/Dicklesworthstone/frankentui/commit/3aa074ad))
- Pane profiling benchmark harness and scripts. ([9595eb1b](https://github.com/Dicklesworthstone/frankentui/commit/9595eb1b))
- Validation pipeline E2E, benchmarks, and profiling (+665 lines). ([f18cec4e](https://github.com/Dicklesworthstone/frankentui/commit/f18cec4e))

### Fixes

- Evidence sink capped at 50 MiB to prevent unbounded disk growth. ([c673643c](https://github.com/Dicklesworthstone/frankentui/commit/c673643c))
- Post-quit subscription reconciliation and event draining guarded. ([69484709](https://github.com/Dicklesworthstone/frankentui/commit/69484709))
- Post-shutdown task rejection with decay-dirtied state persistence. ([d7856692](https://github.com/Dicklesworthstone/frankentui/commit/d7856692))
- SGR sub-parameter flattening for colon-separated sequences. ([a48d33ee](https://github.com/Dicklesworthstone/frankentui/commit/a48d33ee))
- Demo routed to crossterm-compat backend on non-Unix hosts. ([73821127](https://github.com/Dicklesworthstone/frankentui/commit/73821127))
- Pipeline metrics helper extracted; wrap logic skipped for `WrapMode::None`. ([5451ec3d](https://github.com/Dicklesworthstone/frankentui/commit/5451ec3d))

### Refactoring

- Inner-match if-guards converted to match-arm guards across crates. ([d528fd2b](https://github.com/Dicklesworthstone/frankentui/commit/d528fd2b))
- Major README refresh expanding effect system and subscription engine (+1035 lines). ([bd377ca4](https://github.com/Dicklesworthstone/frankentui/commit/bd377ca4))

---

## [v0.2.1] -- 2026-03-07

> GitHub Release: <https://github.com/Dicklesworthstone/frankentui/releases/tag/v0.2.1>
> Published to crates.io as `ftui 0.2.1`.
> Compare: <https://github.com/Dicklesworthstone/frankentui/compare/v0.2.0...v0.2.1>

This release spans a large body of work from the v0.2.0 tag (2026-02-15) through the v0.2.1 tag (2026-03-07), covering 373 commits. It introduced the doctor diagnostic crate, massive widget expansion, formal observability infrastructure, and extensive correctness hardening.

### doctor_frankentui Diagnostic Crate

- New `doctor_frankentui` crate for automated health-check, capture pipeline, and diagnostic reporting. ([dd31dd92](https://github.com/Dicklesworthstone/frankentui/commit/dd31dd92), [b952e774](https://github.com/Dicklesworthstone/frankentui/commit/b952e774))
- Streaming VHS fatal detection, defunct ttyd reaping, and smoke timeout cap. ([76262862](https://github.com/Dicklesworthstone/frankentui/commit/76262862))
- Mapping atlas v2 and v3 with process spawn fallback and abstract interpretation. ([78b29e86](https://github.com/Dicklesworthstone/frankentui/commit/78b29e86), [aab06892](https://github.com/Dicklesworthstone/frankentui/commit/aab06892))
- OpenTUI import pipeline and semantic contract infrastructure. ([66fb6a7b](https://github.com/Dicklesworthstone/frankentui/commit/66fb6a7b))
- Sandbox enforcement for untrusted project analysis; secret detection and artifact redaction. ([c9983c6e](https://github.com/Dicklesworthstone/frankentui/commit/c9983c6e), [c96a316f](https://github.com/Dicklesworthstone/frankentui/commit/c96a316f))

### Widget Expansion

- Galaxy-brain decision card widget with 4-level progressive disclosure transparency layer. ([03427231](https://github.com/Dicklesworthstone/frankentui/commit/03427231), [121a7b66](https://github.com/Dicklesworthstone/frankentui/commit/121a7b66))
- Adaptive radix tree widget with expanded E2E conformance coverage. ([de388072](https://github.com/Dicklesworthstone/frankentui/commit/de388072))
- Choreographic programming for multi-widget interactions. ([13b7dd24](https://github.com/Dicklesworthstone/frankentui/commit/13b7dd24))
- Tabs widget with IME composition, list filtering/multi-select, and palette theming. ([e97cc536](https://github.com/Dicklesworthstone/frankentui/commit/e97cc536))
- Popover widget for anchored floating content. ([e54f496f](https://github.com/Dicklesworthstone/frankentui/commit/e54f496f))
- Table widget stateful persistence layer with comprehensive rendering tests. ([e17a49de](https://github.com/Dicklesworthstone/frankentui/commit/e17a49de))

### Observability and Policy Infrastructure

- Prometheus-compatible metrics registry. ([e0a7eda8](https://github.com/Dicklesworthstone/frankentui/commit/e0a7eda8))
- `slo.yaml` with data-plane and decision-plane budgets. ([12d9e4b5](https://github.com/Dicklesworthstone/frankentui/commit/12d9e4b5))
- `demo.yaml` with 5 reproducible 60-second demos and E2E execution claim verification. ([983ae440](https://github.com/Dicklesworthstone/frankentui/commit/983ae440), [993096f9](https://github.com/Dicklesworthstone/frankentui/commit/993096f9))
- Policy-as-Data progressive delivery and fallback with controller integration tests. ([bfec6a9a](https://github.com/Dicklesworthstone/frankentui/commit/bfec6a9a), [84273439](https://github.com/Dicklesworthstone/frankentui/commit/84273439))
- SOS barrier certificates, CEGIS synthesis, e-graph optimizer, quotient filter, and drift visualization. ([b48eab99](https://github.com/Dicklesworthstone/frankentui/commit/b48eab99))
- JSONL diagnostic logging consolidated around shared event builder + sink abstraction (#32). ([43c7d5d2](https://github.com/Dicklesworthstone/frankentui/commit/43c7d5d2))
- Reusable diagnostic hook/log substrate extracted into `ftui-widgets::diagnostics` (#33). ([04e0146d](https://github.com/Dicklesworthstone/frankentui/commit/04e0146d))

### Terminal and IME

- IME event model (`ImeEvent`, `ImePhase`) added to ftui-core. ([2e93c7f2](https://github.com/Dicklesworthstone/frankentui/commit/2e93c7f2))
- Native IME event pipeline for web, replacing composition-as-paste hack. ([e6cdfed4](https://github.com/Dicklesworthstone/frankentui/commit/e6cdfed4))
- IME composition wired into TextInput; host-focus lifecycle added to FocusManager. ([681aecd0](https://github.com/Dicklesworthstone/frankentui/commit/681aecd0))
- Mouse protocol compatibility hardening, WezTerm mux detection, and legacy mouse parser. ([0af52aa4](https://github.com/Dicklesworthstone/frankentui/commit/0af52aa4))
- C1 control code support for CSI/SS3/OSC sequences. ([e582aad9](https://github.com/Dicklesworthstone/frankentui/commit/e582aad9))

### Text and Typography

- Incremental Knuth-Plass line-break optimizer. ([70f567c9](https://github.com/Dicklesworthstone/frankentui/commit/70f567c9))
- Leading/baseline-grid and paragraph spacing system. ([3a564479](https://github.com/Dicklesworthstone/frankentui/commit/3a564479))
- Microtypographic justification controls. ([e27612b3](https://github.com/Dicklesworthstone/frankentui/commit/e27612b3))
- Layout policy presets and deterministic fallback contract. ([33b28404](https://github.com/Dicklesworthstone/frankentui/commit/33b28404))
- Text shaping and script segmentation modules. ([6b86d038](https://github.com/Dicklesworthstone/frankentui/commit/6b86d038))
- Ligature policy system. ([e6d1ba73](https://github.com/Dicklesworthstone/frankentui/commit/e6d1ba73))
- Tier budget system with Emergency layout tier. ([f3e0f7bc](https://github.com/Dicklesworthstone/frankentui/commit/f3e0f7bc))

### Layout and Panes

- Cache-oblivious van Emde Boas tree layout for widget traversal. ([03b4a8eb](https://github.com/Dicklesworthstone/frankentui/commit/03b4a8eb))
- Persisted workspace schema with validation and migration. ([e61597da](https://github.com/Dicklesworthstone/frankentui/commit/e61597da))
- Dependency graph, incremental engine, and enhanced pane physics. ([cdf6590e](https://github.com/Dicklesworthstone/frankentui/commit/cdf6590e))
- Pane splitter rendering primitives and Layout Lab integration. ([a32769cd](https://github.com/Dicklesworthstone/frankentui/commit/a32769cd))
- Incremental View Maintenance (IVM) with delta-propagation DAG, `IncrementalView` trait, benchmarks, and property tests. ([ee13292b](https://github.com/Dicklesworthstone/frankentui/commit/ee13292b), [34d81f90](https://github.com/Dicklesworthstone/frankentui/commit/34d81f90))
- SmallVec<[T; 8]> in layout solver replacing Vec. ([e960693d](https://github.com/Dicklesworthstone/frankentui/commit/e960693d))

### Runtime

- Graceful signal shutdown, schema compat layer, policy discovery, and buffer safety with compile-fail tests. ([76176973](https://github.com/Dicklesworthstone/frankentui/commit/76176973))
- Bayesian decision core with expected-loss minimization. ([d36dbeae](https://github.com/Dicklesworthstone/frankentui/commit/d36dbeae))
- Predictive screen tick system with persistence. ([1cd5a999](https://github.com/Dicklesworthstone/frankentui/commit/1cd5a999))
- Immediate-drain config, stats, and spin-storm prevention. ([18facd3f](https://github.com/Dicklesworthstone/frankentui/commit/18facd3f))
- Configurable degradation floor to prevent content-suppressing quality levels. ([e507218d](https://github.com/Dicklesworthstone/frankentui/commit/e507218d))
- FrameArena API extended and adopted in TextInput/Dashboard. ([e2e4ba37](https://github.com/Dicklesworthstone/frankentui/commit/e2e4ba37))

### Security

- Threat model, HTTPS-only defaults, and resource bound hardening. ([3125afab](https://github.com/Dicklesworthstone/frankentui/commit/3125afab))
- Clipboard policy API, link audit trail, and wide character handling. ([cddcc986](https://github.com/Dicklesworthstone/frankentui/commit/cddcc986))

### Web/WASM

- WASM gesture handling, text search, layout alignment, and virtualized list rendering expansion. ([f42e6866](https://github.com/Dicklesworthstone/frankentui/commit/f42e6866))
- Interactive pane studio with drag/dock/timeline and engine-backed selection copy. ([720f201d](https://github.com/Dicklesworthstone/frankentui/commit/720f201d))
- Event subscriptions, OSC 9;4 progress signals, and pane gesture extraction for frankenterm-web. ([c2320f0a](https://github.com/Dicklesworthstone/frankentui/commit/c2320f0a))
- Attach state machine hardened against mid-backoff transport closes. ([02bfdf26](https://github.com/Dicklesworthstone/frankentui/commit/02bfdf26))

### Shared Capabilities and Rendering

- SharedCapabilities with ArcSwap-backed concurrent access. ([d70e44cd](https://github.com/Dicklesworthstone/frankentui/commit/d70e44cd))
- SharedResolvedTheme with ArcSwap-backed concurrent access. ([f39037b5](https://github.com/Dicklesworthstone/frankentui/commit/f39037b5))
- Frame guardrails for memory budget and queue depth. ([e3a2d4fb](https://github.com/Dicklesworthstone/frankentui/commit/e3a2d4fb))
- Deterministic fit-to-container and font metric lifecycle. ([0d5d1c47](https://github.com/Dicklesworthstone/frankentui/commit/0d5d1c47))
- Selection copy/rect extraction, AHashMap migration, and render pipeline optimization. ([af044fa7](https://github.com/Dicklesworthstone/frankentui/commit/af044fa7))
- SelectionGestureController for pointer/keyboard gestures. ([792a4543](https://github.com/Dicklesworthstone/frankentui/commit/792a4543))
- `CellAttrs::merged_flags()` for composing style flags. ([05b02705](https://github.com/Dicklesworthstone/frankentui/commit/05b02705))
- Stable slugs added to ScreenMeta with hotkey conflict fix (#31). ([cc0f1d22](https://github.com/Dicklesworthstone/frankentui/commit/cc0f1d22))

### Testing and CI

- Terminal emulator compatibility matrix CI with docs. ([717f2c91](https://github.com/Dicklesworthstone/frankentui/commit/717f2c91))
- Shadow-mode Ratatui comparison across 10 scenarios. ([ecb5602b](https://github.com/Dicklesworthstone/frankentui/commit/ecb5602b))
- Conformance matrix E2E across 5 terminal emulator profiles. ([815119a3](https://github.com/Dicklesworthstone/frankentui/commit/815119a3))
- Full widget gallery E2E test across 5 terminal configs. ([06cf6078](https://github.com/Dicklesworthstone/frankentui/commit/06cf6078))
- Session-type mode transition E2E with JSONL logging. ([3bdd9d21](https://github.com/Dicklesworthstone/frankentui/commit/3bdd9d21))
- Roaring vs Vec<bool> dirty tracking equivalence and benchmark. ([6e7e3a05](https://github.com/Dicklesworthstone/frankentui/commit/6e7e3a05), [28b90ded](https://github.com/Dicklesworthstone/frankentui/commit/28b90ded))
- Layout and widget fuzz targets; Rect::union commutativity bug fixed. ([d6582ce0](https://github.com/Dicklesworthstone/frankentui/commit/d6582ce0))
- EventRecorder and EventReplayer for deterministic replay. ([e37be06c](https://github.com/Dicklesworthstone/frankentui/commit/e37be06c))
- 34 new color themes for comprehensive theme coverage. ([5fc03413](https://github.com/Dicklesworthstone/frankentui/commit/5fc03413))
- Signature-based color harmonization system. ([c5bc61b2](https://github.com/Dicklesworthstone/frankentui/commit/c5bc61b2))

### Correctness and Hardening

- NaN-safe f64 clamping across animation, budget, and widget subsystems. ([da8cb314](https://github.com/Dicklesworthstone/frankentui/commit/da8cb314), [b346386e](https://github.com/Dicklesworthstone/frankentui/commit/b346386e))
- Grapheme pool generation tracking for stale ID detection. ([bfafcf8b](https://github.com/Dicklesworthstone/frankentui/commit/bfafcf8b))
- RAII ReentrancyGuard for two-way binding sync flag. ([911e8e17](https://github.com/Dicklesworthstone/frankentui/commit/911e8e17))
- BiDi cursor algorithm corrections and text handling improvements. ([0662230f](https://github.com/Dicklesworthstone/frankentui/commit/0662230f))
- Focus trap deadlock, sentinel leak, and shutdown hang prevention. ([80f39dc9](https://github.com/Dicklesworthstone/frankentui/commit/80f39dc9))
- Tour panic on empty layout areas guarded. ([97621dda](https://github.com/Dicklesworthstone/frankentui/commit/97621dda))
- Hyperlink registry reset to initial state on terminal reset. ([7f1dd716](https://github.com/Dicklesworthstone/frankentui/commit/7f1dd716))
- Narrow-layout confirm dialog with translucent background compositing. ([5bd1a5ba](https://github.com/Dicklesworthstone/frankentui/commit/5bd1a5ba))
- PTY resize fixed by retaining master FD after spawn. ([b142850b](https://github.com/Dicklesworthstone/frankentui/commit/b142850b))
- File picker root confinement hardened against symlink escapes. ([c819b81b](https://github.com/Dicklesworthstone/frankentui/commit/c819b81b))

### Licensing

- MIT + OpenAI/Anthropic rider adopted across workspace metadata. ([ca794209](https://github.com/Dicklesworthstone/frankentui/commit/ca794209))

---

## [v0.2.0] -- 2026-02-15

> Git tag only (no GitHub Release).
> Tag: <https://github.com/Dicklesworthstone/frankentui/releases/tag/v0.2.0>
> Compare: <https://github.com/Dicklesworthstone/frankentui/compare/v0.1.1...v0.2.0>

This is the first tagged release of FrankenTUI. It spans from the v0.1.1 crates.io publish (2026-02-05) through the version bump on 2026-02-15, covering major feature expansion across all crates.

### FrankenTerm Terminal Engine

- New `frankenterm-core` crate with grid operations, cursor movement, DEC/ANSI modes, and scrollback buffer with resize/reflow. ([db25bf42](https://github.com/Dicklesworthstone/frankentui/commit/db25bf42), [2da057aa](https://github.com/Dicklesworthstone/frankentui/commit/2da057aa), [d5b4c41c](https://github.com/Dicklesworthstone/frankentui/commit/d5b4c41c))
- Incremental patch API with dirty tracking. ([7200ec6d](https://github.com/Dicklesworthstone/frankentui/commit/7200ec6d))
- Cell flags, hyperlink ID, charset translation, and wide-char support. ([7905b327](https://github.com/Dicklesworthstone/frankentui/commit/7905b327), [c151cb3d](https://github.com/Dicklesworthstone/frankentui/commit/c151cb3d))
- VT conformance: HTS/TBC/CBT/ECH/DECKPAM/DECKPNM, DECALN, REP, DECOM, DECSCUSR, DECSTR, and ICH/DCH/IL/DL parser support. ([80380315](https://github.com/Dicklesworthstone/frankentui/commit/80380315), [b7a1361b](https://github.com/Dicklesworthstone/frankentui/commit/b7a1361b), [78529934](https://github.com/Dicklesworthstone/frankentui/commit/78529934))
- New `frankenterm-web` crate with scrollback viewport virtualization, smooth scroll, and plaintext URL auto-link detection. ([c9988ed6](https://github.com/Dicklesworthstone/frankentui/commit/c9988ed6), [3e2c44c8](https://github.com/Dicklesworthstone/frankentui/commit/3e2c44c8), [1af32744](https://github.com/Dicklesworthstone/frankentui/commit/1af32744))

### Backend Abstraction Layer

- New `ftui-backend` trait crate and `ftui-tty` native TTY implementation with Unix raw mode and terminal feature toggles. ([5a314523](https://github.com/Dicklesworthstone/frankentui/commit/5a314523), [ac43f32e](https://github.com/Dicklesworthstone/frankentui/commit/ac43f32e))

### WASM Showcase

- New `ftui-showcase-wasm` crate implementing a WASM showcase runner. ([b1f1303f](https://github.com/Dicklesworthstone/frankentui/commit/b1f1303f))
- Showcase demo HTML for in-browser demonstration. ([68612f63](https://github.com/Dicklesworthstone/frankentui/commit/68612f63))
- wasm32 CI gate and core wasm hygiene (ADR-008). ([d91f1bdc](https://github.com/Dicklesworthstone/frankentui/commit/d91f1bdc))

### Mermaid Diagram Engine

- Full Mermaid diagram rendering pipeline: parser, layout, renderer with support for flowchart, sequence, Gantt, class, and C4 diagram families. ([ad85efc9](https://github.com/Dicklesworthstone/frankentui/commit/ad85efc9), [46600614](https://github.com/Dicklesworthstone/frankentui/commit/46600614))
- Visual diagram diffing (`render_diff`). ([46600614](https://github.com/Dicklesworthstone/frankentui/commit/46600614))
- Gantt bar positioning rewritten to use actual date ranges. ([0d5aeed5](https://github.com/Dicklesworthstone/frankentui/commit/0d5aeed5))
- Comprehensive snapshot tests for all diagram families at multiple viewport sizes. ([8d8bb9e2](https://github.com/Dicklesworthstone/frankentui/commit/8d8bb9e2), [a0ba58da](https://github.com/Dicklesworthstone/frankentui/commit/a0ba58da))
- Mermaid showcase controls and metrics panel. ([a71ac2ae](https://github.com/Dicklesworthstone/frankentui/commit/a71ac2ae))

### Demo Showcase Expansion

- Kanban Board screen. ([46a178ce](https://github.com/Dicklesworthstone/frankentui/commit/46a178ce))
- DragDrop/Quake screens and dashboard splitters. ([a97b6e58](https://github.com/Dicklesworthstone/frankentui/commit/a97b6e58))
- Live Markdown Editor integration. ([4a926ddc](https://github.com/Dicklesworthstone/frankentui/commit/4a926ddc))
- Toggleable FPS render mode in visual effects showcase. ([5762768b](https://github.com/Dicklesworthstone/frankentui/commit/5762768b))
- Mermaid harness telemetry, dense flow sample, and tour landing UI. ([069a80dd](https://github.com/Dicklesworthstone/frankentui/commit/069a80dd))

### Mouse and Input Handling

- Mouse event handling added to Tree, List, and Table widgets. ([65c68a58](https://github.com/Dicklesworthstone/frankentui/commit/65c68a58))
- `Cmd::SetMouseCapture` for runtime mouse capture control. ([9f2c4fa7](https://github.com/Dicklesworthstone/frankentui/commit/9f2c4fa7))
- Hit region registration for chrome overlays and status bar. ([4293bd1d](https://github.com/Dicklesworthstone/frankentui/commit/4293bd1d))
- Mouse dispatcher rewritten to activate on press with release suppression. ([67a37631](https://github.com/Dicklesworthstone/frankentui/commit/67a37631))

### Rendering Improvements

- Presenter orphan detection, O(1) diff lookups, and DiffViewport elimination. ([7e27656e](https://github.com/Dicklesworthstone/frankentui/commit/7e27656e))
- LabelGrid spatial index for mermaid layout performance. ([4f7b777a](https://github.com/Dicklesworthstone/frankentui/commit/4f7b777a))
- Bounds-checked `point_colored_in_bounds` for tight inner loops. ([7640de7d](https://github.com/Dicklesworthstone/frankentui/commit/7640de7d))
- Quality/step branches hoisted outside hot pixel loops in VFX. ([c2d9f6a2](https://github.com/Dicklesworthstone/frankentui/commit/c2d9f6a2))
- Painter clear optimized with generation stamps. ([bd2c8745](https://github.com/Dicklesworthstone/frankentui/commit/bd2c8745))

### Testing Infrastructure

- Over 1,500 new tests across the workspace including edge-case, property, conformance, and snapshot tests.
- 100+ VT conformance fixture tests for frankenterm-core. ([dfee0d76](https://github.com/Dicklesworthstone/frankentui/commit/dfee0d76), [023d479f](https://github.com/Dicklesworthstone/frankentui/commit/023d479f))
- Bayesian subsystem tests: BOCPD, VOI sampling, conformal predictor, height predictor. ([da79feff](https://github.com/Dicklesworthstone/frankentui/commit/da79feff), [cc3a6401](https://github.com/Dicklesworthstone/frankentui/commit/cc3a6401))
- Widget edge-case test suites: Table (519 lines), CommandPalette (43 tests), Virtualized (80 tests), Help (95 tests), Modal (37 tests), and many more. ([a5d18d58](https://github.com/Dicklesworthstone/frankentui/commit/a5d18d58), [2d3b2bf6](https://github.com/Dicklesworthstone/frankentui/commit/2d3b2bf6))
- Flicker detection property tests, terminal model property tests, and i18n/VFX property tests. ([564a237a](https://github.com/Dicklesworthstone/frankentui/commit/564a237a), [21d4df8a](https://github.com/Dicklesworthstone/frankentui/commit/21d4df8a))
- Getting-started guide for library consumers. ([8d4bd83d](https://github.com/Dicklesworthstone/frankentui/commit/8d4bd83d))
- Cx capability context for cancellation and deadline propagation. ([fdd633a1](https://github.com/Dicklesworthstone/frankentui/commit/fdd633a1))

---

## v0.1.1 -- 2026-02-05

> Published to crates.io. No git tag. No GitHub Release.
> Representative commit: [f2e8bca5](https://github.com/Dicklesworthstone/frankentui/commit/f2e8bca5) (publish) and [44a12de1](https://github.com/Dicklesworthstone/frankentui/commit/44a12de1) (version bump).

First crates.io publish of all 13 original workspace crates, bumped from 0.1.0 to 0.1.1 with pinned external dependencies for build reproducibility. Broke the `ftui-extras` / `ftui-harness` publish cycle by inlining ANSI snapshot helpers.

### Included since initial development (2026-01-31 through 2026-02-05)

#### Kernel Architecture and Core

- Rust workspace initialized with multi-crate structure. ([aadc5679](https://github.com/Dicklesworthstone/frankentui/commit/aadc5679))
- Terminal session lifecycle, color downgrade, style system, and terminal model. ([ced1e5e7](https://github.com/Dicklesworthstone/frankentui/commit/ced1e5e7))
- Buffer API with Cell/CellContent/GraphemeId foundations. ([0da0ba05](https://github.com/Dicklesworthstone/frankentui/commit/0da0ba05))
- GraphemePool with reference-counted interning. ([e6cb5b83](https://github.com/Dicklesworthstone/frankentui/commit/e6cb5b83))
- One-writer discipline and inline-mode safety guidance as correctness guardrails. ([d0cd7b5e](https://github.com/Dicklesworthstone/frankentui/commit/d0cd7b5e))
- Inline mode validation helpers and safety improvements. ([3ff7e41a](https://github.com/Dicklesworthstone/frankentui/commit/3ff7e41a))

#### Render Pipeline

- BufferDiff with row-major scan. ([9abb1bf9](https://github.com/Dicklesworthstone/frankentui/commit/9abb1bf9))
- Presenter with state-tracked ANSI emission. ([aa58e858](https://github.com/Dicklesworthstone/frankentui/commit/aa58e858))
- Geometry primitives and drawing module. ([14a19353](https://github.com/Dicklesworthstone/frankentui/commit/14a19353))
- CountingWriter for output bytes tracking. ([07a836e9](https://github.com/Dicklesworthstone/frankentui/commit/07a836e9))
- C1 controls stripped in sanitizer with proptest Rust 2024 compat. ([e2d6e656](https://github.com/Dicklesworthstone/frankentui/commit/e2d6e656))
- Export adapters (HTML, SVG, Text) for Buffer rendering. ([5aa907fc](https://github.com/Dicklesworthstone/frankentui/commit/5aa907fc))

#### Layout

- Flex layout solver. ([0baccfdd](https://github.com/Dicklesworthstone/frankentui/commit/0baccfdd))
- LayoutDebugger with tracing. ([94d9d8bb](https://github.com/Dicklesworthstone/frankentui/commit/94d9d8bb))

#### Runtime

- Elm/Bubbletea-style Program runtime with Model/Cmd pattern, establishing the core event loop. ([75e21361](https://github.com/Dicklesworthstone/frankentui/commit/75e21361))
- Stdio capture utility for accidental println! protection. ([81d16a63](https://github.com/Dicklesworthstone/frankentui/commit/81d16a63))

#### Widget System

- Panel widget with border, title, and padding. ([85d17ace](https://github.com/Dicklesworthstone/frankentui/commit/85d17ace))
- StatusLine widget. ([a58bf932](https://github.com/Dicklesworthstone/frankentui/commit/a58bf932))
- Hit testing and cursor control for interactive widgets. ([ad8cb457](https://github.com/Dicklesworthstone/frankentui/commit/ad8cb457))
- Budget-aware degradation across all widgets. ([21bd6676](https://github.com/Dicklesworthstone/frankentui/commit/21bd6676))
- Composable animation primitives. ([67eb0314](https://github.com/Dicklesworthstone/frankentui/commit/67eb0314))
- Focused input state and CellContent support. ([b9a7fefa](https://github.com/Dicklesworthstone/frankentui/commit/b9a7fefa))
- Virtualized<T> container, LogViewer, hyperlink support. (Multiple commits, 2026-02-01)

#### Text

- ASCII width fast-path optimization. ([86613c83](https://github.com/Dicklesworthstone/frankentui/commit/86613c83))
- Unicode width corpus tests and grapheme helpers. ([bb8b02ab](https://github.com/Dicklesworthstone/frankentui/commit/bb8b02ab))
- Rope text storage and View helpers. ([4fa45b3a](https://github.com/Dicklesworthstone/frankentui/commit/4fa45b3a))

#### Extras

- Console abstraction for styled output. ([39be7b6c](https://github.com/Dicklesworthstone/frankentui/commit/39be7b6c))
- Asciicast v2 session recording. ([491d4732](https://github.com/Dicklesworthstone/frankentui/commit/491d4732))

#### PTY

- PTY signal handling, feature flags, and backpressure fixes. ([748393b7](https://github.com/Dicklesworthstone/frankentui/commit/748393b7))

#### Major Second-Sprint Features (2026-02-02 through 2026-02-04)

- **TextArea, Help, Tree, JsonView, Emoji, Stopwatch, Timer, Pretty widgets** and Live display system.
- **Undo/redo editor core**, Unicode BiDi support, SyntaxHighlighter API.
- **GFM Markdown** rendering with LaTeX and streaming support, plus diagram plumbing.
- **Theme system** and visual-effects primitives (text-effects, gradients, reveal, particle dissolve, metaballs, plasma, wireframe).
- **Responsive layout** primitives: Breakpoints, Responsive<T>, ResponsiveLayout with visibility helpers.
- **Animation stack**: Timeline scheduler, AnimationGroup lifecycle, spring physics, stagger utilities.
- **Reactive runtime**: Observable/Computed values, two-way bindings, BatchScope, undo/redo history, MacroPlayback.
- **TerminalEmulator widget**, key-sequence interpreter, focus management expansion.
- **Theme Studio** with live palette editing and WCAG contrast fixes.
- **Guided Tour system**, Form Validation screen, Virtualized Search screen.
- **Fenwick-tree** variable-height virtualization.
- **Input fairness module** with adaptive scheduling and SLA tracking.
- **Internationalization foundation**: RTL layout mirroring and BiDi tests.
- **Performance HUD** as full screen with real-time metrics.
- **Bayesian intelligence layer**: BOCPD integration into resize coalescer, diff strategy selector, dirty-row diff optimization, VOI telemetry, conformal rank confidence.
- **Evidence sink builder** and allocation budget tracking.
- **Command palette** scoring optimization with evidence descriptions.
- **Saturating arithmetic sweep** across render, text, widgets, layout, and demo screens for overflow safety.
- **Crates.io publish prep** including docs and E2E infrastructure. ([8c327fee](https://github.com/Dicklesworthstone/frankentui/commit/8c327fee))
- **Fuzzing integration** and large test suite expansion.
- **CI/CD pipeline** and Dependabot configuration. ([bee75b14](https://github.com/Dicklesworthstone/frankentui/commit/bee75b14))

---

## v0.1.0 -- 2026-01-31

> Initial version. No publish. No tag.

- Initial commit with FrankenTUI plan documents and architectural design (V5/V6.1 hybrid architecture). ([7a23b45a](https://github.com/Dicklesworthstone/frankentui/commit/7a23b45a))
- Comprehensive bead graph with dependency structure and implementation roadmap covering 15 feature areas with 46 subtasks.
- Reference library sync script and build infrastructure.

---

[Unreleased]: https://github.com/Dicklesworthstone/frankentui/compare/v0.2.1...main
[v0.2.1]: https://github.com/Dicklesworthstone/frankentui/compare/v0.2.0...v0.2.1
[v0.2.0]: https://github.com/Dicklesworthstone/frankentui/releases/tag/v0.2.0

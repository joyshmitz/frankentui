#![forbid(unsafe_code)]

//! `doctor_frankentui` is the operator-facing workflow crate for capture,
//! certification, replay, suite execution, and migration planning.
//!
//! The crate deliberately mixes pure analysis modules with a small set of
//! orchestration-heavy command paths. Those orchestration paths are the primary
//! candidates for supervised Asupersync migration because they own blocking I/O,
//! retries, subprocess lifecycles, timeouts, evidence writes, and fallback
//! behavior.
//!
//! # Command Surface
//!
//! [`cli::run`] dispatches the operator-visible commands:
//!
//! - `replay` (`capture.rs`): full replay/capture workflow for a single profile.
//! - `seed-demo` (`seed.rs`): MCP bootstrap + demo message seeding.
//! - `migrate` (`suite.rs`): repeated replay runs plus manifest/report synthesis.
//! - `certify` (`doctor.rs`): environment and capture-stack certification with
//!   degraded-mode fallback.
//! - `report` (`report.rs`): artifact/report synthesis over completed suite runs.
//! - `plan` (`import.rs`): import-intake planning and snapshot materialization.
//!
//! # Workflow Inventory
//!
//! The orchestration topology is concentrated in a few modules:
//!
//! - [`seed`]: blocking HTTP/MCP bootstrap, retry policy, backoff sleeps, and
//!   server-readiness polling before project/agent/message setup.
//! - [`capture`]: the heaviest workflow. It resolves profile/config state,
//!   optionally starts seed-demo, prepares tapes/artifacts, spawns replay/VHS/tmux
//!   subprocesses, waits on timeouts, records decision ledgers, and finalizes
//!   media/snapshot artifacts.
//! - [`doctor`]: certification wrapper around command checks, capture smoke runs,
//!   degraded-capture classification, app-launch fallback, and summary emission.
//! - [`suite`]: profile fan-out, per-run process orchestration via the CLI,
//!   manifest/index generation, and final report invocation.
//! - [`report`]: post-processing over existing artifacts; lower supervision value
//!   than `capture`/`doctor`, but still part of the end-to-end operator contract.
//! - [`util`], [`runmeta`], [`tape`], and [`profile`]: shared plumbing for
//!   filesystem writes, timestamps, tape generation, and reproducible metadata.
//!
//! The rest of the crate is mostly pure translation/analysis logic and should
//! remain synchronous unless a later change proves otherwise.
//!
//! # Proposed Supervised Topology
//!
//! The migration target is a per-command supervision tree with explicit child
//! responsibilities and cancellation boundaries:
//!
//! 1. Root command scope
//!    Owns CLI args, run IDs, output mode, shared evidence sinks, and global
//!    deadline/cancellation propagation.
//! 2. Network/bootstrap lane
//!    Used by `seed-demo` and any capture paths that call it. Responsible for
//!    MCP health checks, bounded retries, backoff timing, and request/response
//!    logging.
//! 3. Subprocess lane
//!    Used by `capture`, `doctor`, and `suite`. Responsible for spawning,
//!    stdout/stderr capture, timeout enforcement, observer/tmux lifecycle, exit
//!    classification, and cleanup on cancellation.
//! 4. Artifact lane
//!    Responsible for run directory creation, summary/manifests/runmeta writes,
//!    append-only ledgers, and redaction-friendly evidence persistence.
//! 5. Aggregation lane
//!    Used by `suite`/`report` to merge per-run outcomes into manifest, summary,
//!    and operator-facing report artifacts.
//!
//! # Migration Sequence
//!
//! The code suggests a clear order of operations:
//!
//! 1. Inventory/topology definition in this crate-level doc and related tests.
//! 2. `seed.rs` migration first: smallest orchestration surface with concrete
//!    retry/wait logic and low blast radius.
//! 3. `capture.rs` and `doctor.rs` subprocess supervision next: they hold the
//!    highest operator pain and the richest fallback/timeout behavior.
//! 4. `suite.rs` fan-out/report composition after the lower-level lanes are
//!    supervised.
//! 5. Validation expansion last, once the new orchestration boundaries are
//!    stable enough to test deterministically.
//!
//! # Invariants Worth Preserving
//!
//! Any migration should preserve these crate-level contracts:
//!
//! - CLI behavior and output schemas remain stable.
//! - All deadlines are explicit and observable in artifacts/logs.
//! - Cancellation and degraded-mode exits still emit actionable evidence.
//! - Replay/capture runs keep deterministic run IDs, manifests, and summary
//!   paths suitable for local triage and CI uploads.

pub mod abstract_interpretation;
pub mod adversarial_fixtures;
pub mod backend_capability;
pub mod capability_gap;
pub mod capture;
pub mod cegis_synthesis;
pub mod cli;
pub mod code_emission;
pub mod codegen_optimize;
pub mod composition_semantics;
pub mod corpus;
pub mod coverage_prioritizer;
pub mod doctor;
pub mod effect_canonical;
pub mod effect_translator;
pub mod egraph_optimizer;
pub mod error;
pub mod explain;
pub mod fixture_taxonomy;
pub mod gap_triage;
pub mod harness;
pub mod import;
pub mod intent_inference;
pub mod ir_explainer;
pub mod ir_normalize;
pub mod ir_versioning;
pub mod keyseq;
pub mod lowering;
pub mod mapping_atlas;
pub mod migration_config;
pub mod migration_ir;
pub mod module_graph;
pub mod paper_verification;
pub mod profile;
pub mod redact;
pub mod report;
pub mod runmeta;
pub mod sandbox;
pub mod seed;
pub mod semantic_contract;
pub mod state_effects;
pub mod state_event_translator;
pub mod style_semantics;
pub mod style_translator;
pub mod suite;
pub mod tape;
pub mod trace;
pub mod translation_planner;
pub mod tsx_parser;
pub mod util;
pub mod view_layout_translator;

pub use cli::run_from_env;
pub use error::{DoctorError, Result};

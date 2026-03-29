//! Shadow-run comparator for legacy vs structured cancellation lanes (bd-1tznn).
//!
//! Exercises identical workloads through both runtime lanes and compares outputs.
//! Mismatches produce detailed evidence showing exactly where behavior diverged.
//!
//! The comparator uses `ProgramSimulator` as the deterministic execution engine
//! and `RuntimeLane` to label which lane is under test.

#![forbid(unsafe_code)]

use ftui_core::event::Event;
use ftui_harness::failure_signatures::FailureClass;
use ftui_harness::validation_matrix::AssertionCategory;
use ftui_render::frame::Frame;
use ftui_runtime::program::{Cmd, Model, RuntimeLane};
use ftui_runtime::simulator::ProgramSimulator;
use serde::Serialize;
use std::time::Duration;

// ============================================================================
// Shadow comparison infrastructure
// ============================================================================

/// Result of running a single scenario through a lane.
#[derive(Debug, Clone)]
struct LaneResult {
    lane: RuntimeLane,
    trace: Vec<String>,
    logs: Vec<String>,
    running: bool,
    tick_rate: Option<Duration>,
    cmd_log_len: usize,
    frame_hashes: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MismatchReasonCode {
    Trace,
    Log,
    RunningState,
    TickRate,
    CommandLogLength,
    FrameHash,
    FrameCount,
}

impl MismatchReasonCode {
    const fn code(self) -> &'static str {
        match self {
            Self::Trace => "TRACE_DIVERGENCE",
            Self::Log => "LOG_DIVERGENCE",
            Self::RunningState => "RUNNING_STATE_DIVERGENCE",
            Self::TickRate => "TICK_RATE_DIVERGENCE",
            Self::CommandLogLength => "COMMAND_LOG_LENGTH_DIVERGENCE",
            Self::FrameHash => "FRAME_HASH_DIVERGENCE",
            Self::FrameCount => "FRAME_COUNT_DIVERGENCE",
        }
    }

    const fn root_cause_class(self) -> &'static str {
        match self {
            Self::Trace | Self::RunningState | Self::FrameHash | Self::FrameCount => "semantic",
            Self::Log | Self::CommandLogLength => "observability",
            Self::TickRate => "policy",
        }
    }

    const fn failure_class(self) -> FailureClass {
        match self {
            Self::Trace | Self::RunningState | Self::FrameHash | Self::FrameCount => {
                FailureClass::ShadowDivergence
            }
            Self::Log | Self::CommandLogLength => FailureClass::Mismatch,
            Self::TickRate => FailureClass::Rollback,
        }
    }
}

/// Evidence bundle for a shadow-run mismatch.
#[derive(Debug, Clone, Serialize)]
struct MismatchEvidence {
    reason_code: &'static str,
    failure_class: &'static str,
    root_cause_class: &'static str,
    field: String,
    legacy: String,
    structured: String,
    scenario: String,
    summary: String,
}

impl std::fmt::Display for MismatchEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}:{}] '{}' in scenario '{}':\n  legacy:     {}\n  structured: {}",
            self.reason_code,
            self.root_cause_class,
            self.field,
            self.scenario,
            self.legacy,
            self.structured
        )
    }
}

#[derive(Debug, Clone, Serialize)]
struct LaneSummary {
    lane: String,
    trace_len: usize,
    log_len: usize,
    running: bool,
    tick_rate_ms: Option<u64>,
    cmd_log_len: usize,
    frame_hashes: Vec<u64>,
}

#[derive(Debug, Clone, Copy)]
struct ScenarioSpec {
    name: &'static str,
    scenario_kind: &'static str,
    contract_focus: &'static str,
    assertion: AssertionCategory,
    messages: fn() -> Vec<SMsg>,
    frames: &'static [(u16, u16)],
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioReport {
    schema_version: &'static str,
    scenario: String,
    scenario_kind: &'static str,
    contract_focus: &'static str,
    assertion_category: &'static str,
    verdict: &'static str,
    contract_status: &'static str,
    acceptable_difference_policy: &'static str,
    replay_command: String,
    baseline: LaneSummary,
    candidate: LaneSummary,
    mismatch_count: usize,
    mismatches: Vec<MismatchEvidence>,
}

#[derive(Debug, Clone, Serialize)]
struct SuiteSummary {
    total_scenarios: usize,
    matched_scenarios: usize,
    diverged_scenarios: usize,
    total_mismatches: usize,
    scenario_filter: String,
}

#[derive(Debug, Clone, Serialize)]
struct RuntimeShadowSuiteReport {
    schema_version: &'static str,
    suite: &'static str,
    user_contract: &'static str,
    summary: SuiteSummary,
    scenarios: Vec<ScenarioReport>,
}

fn mismatch_reason(field: &str) -> MismatchReasonCode {
    match field {
        "trace" => MismatchReasonCode::Trace,
        "logs" => MismatchReasonCode::Log,
        "running" => MismatchReasonCode::RunningState,
        "tick_rate" => MismatchReasonCode::TickRate,
        "cmd_log_len" => MismatchReasonCode::CommandLogLength,
        "frame_hashes" => MismatchReasonCode::FrameHash,
        "frame_count" => MismatchReasonCode::FrameCount,
        _ => MismatchReasonCode::Trace,
    }
}

fn push_mismatch(
    mismatches: &mut Vec<MismatchEvidence>,
    scenario: &str,
    field: &str,
    legacy: String,
    structured: String,
) {
    let reason = mismatch_reason(field);
    mismatches.push(MismatchEvidence {
        reason_code: reason.code(),
        failure_class: reason.failure_class().reason_code(),
        root_cause_class: reason.root_cause_class(),
        field: field.into(),
        legacy,
        structured,
        scenario: scenario.into(),
        summary: format!(
            "{field} mismatch in scenario '{scenario}'; semantic and policy drift are blockers, observability drift is a blocker when replay context changes"
        ),
    });
}

/// Compare two lane results and return mismatch evidence if any.
fn compare_results(
    scenario: &str,
    legacy: &LaneResult,
    structured: &LaneResult,
) -> Vec<MismatchEvidence> {
    let mut mismatches = Vec::new();

    if legacy.trace != structured.trace {
        push_mismatch(
            &mut mismatches,
            scenario,
            "trace",
            format!("{:?}", legacy.trace),
            format!("{:?}", structured.trace),
        );
    }

    if legacy.logs != structured.logs {
        push_mismatch(
            &mut mismatches,
            scenario,
            "logs",
            format!("{:?}", legacy.logs),
            format!("{:?}", structured.logs),
        );
    }

    if legacy.running != structured.running {
        push_mismatch(
            &mut mismatches,
            scenario,
            "running",
            legacy.running.to_string(),
            structured.running.to_string(),
        );
    }

    if legacy.tick_rate != structured.tick_rate {
        push_mismatch(
            &mut mismatches,
            scenario,
            "tick_rate",
            format!("{:?}", legacy.tick_rate),
            format!("{:?}", structured.tick_rate),
        );
    }

    if legacy.cmd_log_len != structured.cmd_log_len {
        push_mismatch(
            &mut mismatches,
            scenario,
            "cmd_log_len",
            legacy.cmd_log_len.to_string(),
            structured.cmd_log_len.to_string(),
        );
    }

    if legacy.frame_hashes != structured.frame_hashes {
        push_mismatch(
            &mut mismatches,
            scenario,
            "frame_hashes",
            format!("{:?}", legacy.frame_hashes),
            format!("{:?}", structured.frame_hashes),
        );
    }

    mismatches
}

/// Hash a buffer's content for quick comparison.
fn hash_buffer(buf: &ftui_render::buffer::Buffer) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    buf.width().hash(&mut hasher);
    buf.height().hash(&mut hasher);
    for y in 0..buf.height() {
        for x in 0..buf.width() {
            if let Some(cell) = buf.get(x, y) {
                cell.content.as_char().hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

// ============================================================================
// Shadow model: records trace for comparison
// ============================================================================

struct ShadowModel {
    trace: Vec<String>,
}

impl ShadowModel {
    fn new() -> Self {
        Self { trace: vec![] }
    }
}

#[derive(Debug)]
enum SMsg {
    Init,
    Step(String),
    Batch(Vec<String>),
    Sequence(Vec<String>),
    Nested(u32),
    Task(String),
    TaskResult(String),
    Log(String),
    Tick,
    Quit,
    QuitInBatch(usize),
}

impl From<Event> for SMsg {
    fn from(_: Event) -> Self {
        Self::Step("event".into())
    }
}

impl Model for ShadowModel {
    type Message = SMsg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.trace.push("init".into());
        Cmd::msg(SMsg::Init)
    }

    fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
        match msg {
            SMsg::Init => {
                self.trace.push("update:init".into());
                Cmd::none()
            }
            SMsg::Step(step) => {
                self.trace.push(format!("step:{step}"));
                Cmd::none()
            }
            SMsg::Batch(items) => {
                self.trace.push(format!("batch:{}", items.len()));
                Cmd::batch(
                    items
                        .into_iter()
                        .map(|item| Cmd::msg(SMsg::Step(item)))
                        .collect(),
                )
            }
            SMsg::Sequence(items) => {
                self.trace.push(format!("seq:{}", items.len()));
                Cmd::sequence(
                    items
                        .into_iter()
                        .map(|item| Cmd::msg(SMsg::Step(item)))
                        .collect(),
                )
            }
            SMsg::Nested(depth) => {
                self.trace.push(format!("nested:{depth}"));
                if depth > 0 {
                    Cmd::msg(SMsg::Nested(depth - 1))
                } else {
                    Cmd::none()
                }
            }
            SMsg::Task(label) => {
                self.trace.push(format!("task:{label}"));
                let task_label = label.clone();
                Cmd::task(move || SMsg::TaskResult(task_label))
            }
            SMsg::TaskResult(label) => {
                self.trace.push(format!("task-done:{label}"));
                Cmd::none()
            }
            SMsg::Log(text) => {
                self.trace.push(format!("log:{text}"));
                Cmd::log(text)
            }
            SMsg::Tick => {
                self.trace.push("tick".into());
                Cmd::tick(Duration::from_millis(100))
            }
            SMsg::Quit => {
                self.trace.push("quit".into());
                Cmd::quit()
            }
            SMsg::QuitInBatch(count) => {
                self.trace.push(format!("quit-batch:{count}"));
                let mut commands: Vec<Cmd<SMsg>> = (0..count)
                    .map(|idx| Cmd::msg(SMsg::Step(format!("pre-{idx}"))))
                    .collect();
                commands.push(Cmd::quit());
                commands.push(Cmd::msg(SMsg::Step("post-quit".into())));
                Cmd::batch(commands)
            }
        }
    }

    fn view(&self, frame: &mut Frame) {
        let text = format!("n={}", self.trace.len());
        for (idx, ch) in text.chars().enumerate() {
            if (idx as u16) < frame.width() {
                use ftui_render::cell::Cell;
                frame.buffer.set_raw(idx as u16, 0, Cell::from_char(ch));
            }
        }
    }
}

/// Run a scenario through a specific lane and capture results.
fn run_lane(lane: RuntimeLane, msgs: Vec<SMsg>, capture_frames: &[(u16, u16)]) -> LaneResult {
    let mut sim = ProgramSimulator::new(ShadowModel::new());
    sim.init();

    for msg in msgs {
        sim.send(msg);
    }

    let mut frame_hashes = Vec::new();
    for &(width, height) in capture_frames {
        let buf = sim.capture_frame(width, height);
        frame_hashes.push(hash_buffer(buf));
    }

    LaneResult {
        lane,
        trace: sim.model().trace.clone(),
        logs: sim.logs().to_vec(),
        running: sim.is_running(),
        tick_rate: sim.tick_rate(),
        cmd_log_len: sim.command_log().len(),
        frame_hashes,
    }
}

fn lane_summary(result: &LaneResult) -> LaneSummary {
    LaneSummary {
        lane: result.lane.label().to_string(),
        trace_len: result.trace.len(),
        log_len: result.logs.len(),
        running: result.running,
        tick_rate_ms: result
            .tick_rate
            .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
        cmd_log_len: result.cmd_log_len,
        frame_hashes: result.frame_hashes.clone(),
    }
}

fn replay_command_for(scenario: &str) -> String {
    format!("scripts/runtime_shadow_compare.sh /tmp/ftui_runtime_shadow_replay {scenario}")
}

fn scenario_basic_steps() -> Vec<SMsg> {
    vec![
        SMsg::Step("a".into()),
        SMsg::Step("b".into()),
        SMsg::Step("c".into()),
    ]
}

fn scenario_complex_burst() -> Vec<SMsg> {
    vec![
        SMsg::Step("start".into()),
        SMsg::Batch(vec!["b1".into(), "b2".into()]),
        SMsg::Task("compute".into()),
        SMsg::Nested(5),
        SMsg::Log("checkpoint".into()),
        SMsg::Sequence(vec!["s1".into(), "s2".into()]),
        SMsg::Task("finalize".into()),
        SMsg::Tick,
    ]
}

fn scenario_quit_stops() -> Vec<SMsg> {
    vec![
        SMsg::Step("before".into()),
        SMsg::Quit,
        SMsg::Step("after".into()),
    ]
}

fn scenario_quit_in_batch() -> Vec<SMsg> {
    vec![SMsg::QuitInBatch(3)]
}

fn scenario_log_output() -> Vec<SMsg> {
    vec![SMsg::Log("hello".into()), SMsg::Log("world".into())]
}

fn scenario_empty() -> Vec<SMsg> {
    Vec::new()
}

fn scenario_saturation() -> Vec<SMsg> {
    let mut messages = Vec::new();
    messages.push(SMsg::Batch(
        (0..96).map(|idx| format!("burst-{idx}")).collect(),
    ));
    for idx in 0..12 {
        messages.push(SMsg::Task(format!("task-{idx}")));
    }
    for idx in 0..8 {
        messages.push(SMsg::Sequence(vec![
            format!("seq-{idx}-a"),
            format!("seq-{idx}-b"),
            format!("seq-{idx}-c"),
        ]));
    }
    messages.push(SMsg::Nested(24));
    messages.push(SMsg::Tick);
    messages
}

const FRAMES_SMALL: &[(u16, u16)] = &[(40, 10)];
const FRAMES_COMPLEX: &[(u16, u16)] = &[(80, 24), (40, 10)];
const FRAMES_EMPTY: &[(u16, u16)] = &[(10, 5)];
const FRAMES_SATURATION: &[(u16, u16)] = &[(120, 40), (80, 24), (40, 10)];

fn operator_scenarios() -> Vec<ScenarioSpec> {
    vec![
        ScenarioSpec {
            name: "steady_basic_steps",
            scenario_kind: "steady_state",
            contract_focus: "semantic_ordering",
            assertion: AssertionCategory::NoChange,
            messages: scenario_basic_steps,
            frames: FRAMES_SMALL,
        },
        ScenarioSpec {
            name: "bursty_complex",
            scenario_kind: "bursty",
            contract_focus: "degraded_mode_recovery",
            assertion: AssertionCategory::NoRegression,
            messages: scenario_complex_burst,
            frames: FRAMES_COMPLEX,
        },
        ScenarioSpec {
            name: "cancellation_quit_stops",
            scenario_kind: "cancellation_heavy",
            contract_focus: "cancellation_cutoff",
            assertion: AssertionCategory::NoRegression,
            messages: scenario_quit_stops,
            frames: &[],
        },
        ScenarioSpec {
            name: "shutdown_quit_in_batch",
            scenario_kind: "shutdown_heavy",
            contract_focus: "shutdown_draining",
            assertion: AssertionCategory::GracefulFallback,
            messages: scenario_quit_in_batch,
            frames: &[],
        },
        ScenarioSpec {
            name: "observability_logs",
            scenario_kind: "negative_control",
            contract_focus: "observability_replay_context",
            assertion: AssertionCategory::FailureForensics,
            messages: scenario_log_output,
            frames: &[],
        },
        ScenarioSpec {
            name: "negative_control_empty",
            scenario_kind: "negative_control",
            contract_focus: "stable_noop_behavior",
            assertion: AssertionCategory::NoChange,
            messages: scenario_empty,
            frames: FRAMES_EMPTY,
        },
        ScenarioSpec {
            name: "saturation_burst_load",
            scenario_kind: "saturation",
            contract_focus: "load_envelope_and_recovery",
            assertion: AssertionCategory::GracefulFallback,
            messages: scenario_saturation,
            frames: FRAMES_SATURATION,
        },
    ]
}

fn select_operator_scenarios() -> Vec<ScenarioSpec> {
    let filter = std::env::var("FTUI_RUNTIME_SHADOW_SCENARIO").ok();
    let scenarios = operator_scenarios();
    match filter
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None | Some("all") => scenarios,
        Some(name) => {
            let selected = scenarios
                .into_iter()
                .filter(|scenario| scenario.name == name)
                .collect::<Vec<_>>();
            assert!(
                !selected.is_empty(),
                "unknown FTUI_RUNTIME_SHADOW_SCENARIO={name}; available: {}",
                operator_scenarios()
                    .iter()
                    .map(|scenario| scenario.name)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            selected
        }
    }
}

fn build_scenario_report(spec: &ScenarioSpec) -> ScenarioReport {
    let legacy = run_lane(RuntimeLane::Legacy, (spec.messages)(), spec.frames);
    let structured = run_lane(RuntimeLane::Structured, (spec.messages)(), spec.frames);
    let mut mismatches = compare_results(spec.name, &legacy, &structured);
    if legacy.frame_hashes.len() != structured.frame_hashes.len() {
        push_mismatch(
            &mut mismatches,
            spec.name,
            "frame_count",
            legacy.frame_hashes.len().to_string(),
            structured.frame_hashes.len().to_string(),
        );
    }
    ScenarioReport {
        schema_version: "ftui-runtime-shadow-v1",
        scenario: spec.name.to_string(),
        scenario_kind: spec.scenario_kind,
        contract_focus: spec.contract_focus,
        assertion_category: spec.assertion.label(),
        verdict: if mismatches.is_empty() {
            "match"
        } else {
            "diverged"
        },
        contract_status: if mismatches.is_empty() {
            "within-contract"
        } else {
            "out-of-contract"
        },
        acceptable_difference_policy: "Semantic, policy, and replay-context differences are blockers; bounded graceful-fallback differences must still preserve the declared degraded-mode and recovery contract.",
        replay_command: replay_command_for(spec.name),
        baseline: lane_summary(&legacy),
        candidate: lane_summary(&structured),
        mismatch_count: mismatches.len(),
        mismatches,
    }
}

fn build_operator_suite_report() -> RuntimeShadowSuiteReport {
    let scenario_filter = std::env::var("FTUI_RUNTIME_SHADOW_SCENARIO")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "all".to_string());
    let scenarios = select_operator_scenarios()
        .into_iter()
        .map(|scenario| build_scenario_report(&scenario))
        .collect::<Vec<_>>();
    let diverged_scenarios = scenarios
        .iter()
        .filter(|scenario| scenario.verdict == "diverged")
        .count();
    let total_mismatches = scenarios
        .iter()
        .map(|scenario| scenario.mismatch_count)
        .sum();
    RuntimeShadowSuiteReport {
        schema_version: "ftui-runtime-shadow-suite-v1",
        suite: "runtime_shadow_comparison",
        user_contract: "Shadow and saturation comparison must preserve user-visible degraded-mode, recovery, shutdown, and replayability guarantees; mismatches must carry reason codes and replay commands.",
        summary: SuiteSummary {
            total_scenarios: scenarios.len(),
            matched_scenarios: scenarios.len().saturating_sub(diverged_scenarios),
            diverged_scenarios,
            total_mismatches,
            scenario_filter,
        },
        scenarios,
    }
}

fn emit_operator_suite_report(report: &RuntimeShadowSuiteReport) {
    if std::env::var("FTUI_RUNTIME_SHADOW_EMIT_REPORT")
        .ok()
        .as_deref()
        == Some("1")
    {
        println!(
            "FTUI_RUNTIME_SHADOW_REPORT_JSON={}",
            serde_json::to_string(report).expect("serialize runtime shadow suite report")
        );
    }
}

/// Run a scenario through both lanes and assert no mismatches.
fn shadow_compare(scenario: &str, msgs_fn: impl Fn() -> Vec<SMsg>, frames: &[(u16, u16)]) {
    let legacy = run_lane(RuntimeLane::Legacy, msgs_fn(), frames);
    let structured = run_lane(RuntimeLane::Structured, msgs_fn(), frames);
    let mismatches = compare_results(scenario, &legacy, &structured);
    assert!(
        mismatches.is_empty(),
        "Shadow-run mismatches detected:\n{}",
        mismatches
            .iter()
            .map(|mismatch| format!("  {mismatch}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ============================================================================
// SHADOW-RUN TESTS: verify lane equivalence
// ============================================================================

#[test]
fn shadow_basic_steps() {
    shadow_compare("basic_steps", scenario_basic_steps, FRAMES_SMALL);
}

#[test]
fn shadow_batch_ordering() {
    shadow_compare(
        "batch_ordering",
        || vec![SMsg::Batch(vec!["x".into(), "y".into(), "z".into()])],
        FRAMES_SMALL,
    );
}

#[test]
fn shadow_sequence_ordering() {
    shadow_compare(
        "sequence_ordering",
        || vec![SMsg::Sequence(vec!["p".into(), "q".into(), "r".into()])],
        FRAMES_SMALL,
    );
}

#[test]
fn shadow_task_execution() {
    shadow_compare(
        "task_execution",
        || vec![SMsg::Task("alpha".into()), SMsg::Task("beta".into())],
        FRAMES_SMALL,
    );
}

#[test]
fn shadow_nested_recursion() {
    shadow_compare("nested_recursion", || vec![SMsg::Nested(10)], &[]);
}

#[test]
fn shadow_log_output() {
    shadow_compare("log_output", scenario_log_output, &[]);
}

#[test]
fn shadow_tick_rate() {
    shadow_compare("tick_rate", || vec![SMsg::Tick], &[]);
}

#[test]
fn shadow_quit_stops_processing() {
    shadow_compare("quit_stops", scenario_quit_stops, &[]);
}

#[test]
fn shadow_quit_in_batch() {
    shadow_compare("quit_in_batch", scenario_quit_in_batch, &[]);
}

#[test]
fn shadow_complex_scenario() {
    shadow_compare("complex", scenario_complex_burst, FRAMES_COMPLEX);
}

#[test]
fn shadow_multiple_frame_captures() {
    shadow_compare(
        "multi_frame",
        || {
            vec![
                SMsg::Step("frame-1".into()),
                SMsg::Step("frame-2".into()),
                SMsg::Step("frame-3".into()),
            ]
        },
        &[(20, 5), (40, 10), (80, 24)],
    );
}

#[test]
fn shadow_empty_scenario() {
    shadow_compare("empty", scenario_empty, FRAMES_EMPTY);
}

#[test]
fn shadow_large_batch() {
    shadow_compare(
        "large_batch",
        || {
            let items: Vec<String> = (0..50).map(|idx| format!("item-{idx}")).collect();
            vec![SMsg::Batch(items)]
        },
        &[(80, 24)],
    );
}

// ============================================================================
// OPERATOR REPORTS: structured artifacts for shadow and saturation suites
// ============================================================================

#[test]
fn mismatch_reason_codes_cover_runtime_fields() {
    let cases = [
        ("trace", "TRACE_DIVERGENCE", "semantic", "SHADOW_DIVERGENCE"),
        ("logs", "LOG_DIVERGENCE", "observability", "MISMATCH"),
        (
            "running",
            "RUNNING_STATE_DIVERGENCE",
            "semantic",
            "SHADOW_DIVERGENCE",
        ),
        ("tick_rate", "TICK_RATE_DIVERGENCE", "policy", "ROLLBACK"),
        (
            "cmd_log_len",
            "COMMAND_LOG_LENGTH_DIVERGENCE",
            "observability",
            "MISMATCH",
        ),
        (
            "frame_hashes",
            "FRAME_HASH_DIVERGENCE",
            "semantic",
            "SHADOW_DIVERGENCE",
        ),
        (
            "frame_count",
            "FRAME_COUNT_DIVERGENCE",
            "semantic",
            "SHADOW_DIVERGENCE",
        ),
    ];

    for (field, code, root_cause, failure_class) in cases {
        let reason = mismatch_reason(field);
        assert_eq!(reason.code(), code);
        assert_eq!(reason.root_cause_class(), root_cause);
        assert_eq!(reason.failure_class().reason_code(), failure_class);
    }
}

#[test]
fn shadow_runtime_operator_report_contains_replay_commands() {
    let report = build_operator_suite_report();
    assert!(report.summary.total_scenarios >= 6);
    assert_eq!(report.summary.total_scenarios, report.scenarios.len());
    assert!(
        report
            .scenarios
            .iter()
            .any(|scenario| scenario.scenario_kind == "saturation"),
        "suite should include a saturation scenario"
    );
    for scenario in &report.scenarios {
        assert!(
            !scenario.assertion_category.is_empty(),
            "assertion category missing for {}",
            scenario.scenario
        );
        assert_eq!(scenario.contract_status, "within-contract");
        assert!(
            scenario
                .replay_command
                .contains("scripts/runtime_shadow_compare.sh"),
            "replay command missing operator script for {}",
            scenario.scenario
        );
    }
}

#[test]
fn shadow_runtime_operator_artifacts() {
    let report = build_operator_suite_report();
    emit_operator_suite_report(&report);
    assert_eq!(
        report.summary.diverged_scenarios,
        0,
        "runtime shadow suite diverged:\n{}",
        report
            .scenarios
            .iter()
            .filter(|scenario| scenario.verdict == "diverged")
            .flat_map(|scenario| scenario
                .mismatches
                .iter()
                .map(std::string::ToString::to_string))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ============================================================================
// DETERMINISM: shadow comparison must be stable across runs
// ============================================================================

#[test]
fn shadow_deterministic_across_multiple_runs() {
    let mut results = Vec::new();
    for _ in 0..5 {
        let legacy = run_lane(
            RuntimeLane::Legacy,
            vec![
                SMsg::Step("a".into()),
                SMsg::Batch(vec!["b".into(), "c".into()]),
                SMsg::Task("t".into()),
                SMsg::Log("l".into()),
            ],
            FRAMES_SMALL,
        );
        results.push(legacy);
    }

    for (idx, result) in results.iter().enumerate().skip(1) {
        assert_eq!(result.trace, results[0].trace, "run {idx} trace diverged");
        assert_eq!(
            result.frame_hashes, results[0].frame_hashes,
            "run {idx} frame hashes diverged"
        );
    }
}

// ============================================================================
// LANE IDENTITY: verify RuntimeLane metadata
// ============================================================================

#[test]
fn runtime_lane_labels() {
    assert_eq!(RuntimeLane::Legacy.label(), "legacy");
    assert_eq!(RuntimeLane::Structured.label(), "structured");
    assert_eq!(RuntimeLane::Asupersync.label(), "asupersync");
}

#[test]
fn runtime_lane_resolve_fallback() {
    assert_eq!(RuntimeLane::Legacy.resolve(), RuntimeLane::Legacy);
    assert_eq!(RuntimeLane::Structured.resolve(), RuntimeLane::Structured);
    // Asupersync falls back to Structured (not yet implemented)
    assert_eq!(RuntimeLane::Asupersync.resolve(), RuntimeLane::Structured);
}

#[test]
fn runtime_lane_structured_cancellation_check() {
    assert!(!RuntimeLane::Legacy.uses_structured_cancellation());
    assert!(RuntimeLane::Structured.uses_structured_cancellation());
    assert!(RuntimeLane::Asupersync.uses_structured_cancellation());
}

#[test]
fn runtime_lane_default_is_structured() {
    assert_eq!(RuntimeLane::default(), RuntimeLane::Structured);
}

#[test]
fn runtime_lane_display() {
    assert_eq!(format!("{}", RuntimeLane::Legacy), "legacy");
    assert_eq!(format!("{}", RuntimeLane::Structured), "structured");
    assert_eq!(format!("{}", RuntimeLane::Asupersync), "asupersync");
}

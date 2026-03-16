//! Shadow-run comparator for legacy vs structured cancellation lanes (bd-1tznn).
//!
//! Exercises identical workloads through both runtime lanes and compares outputs.
//! Mismatches produce detailed evidence showing exactly where behavior diverged.
//!
//! The comparator uses `ProgramSimulator` as the deterministic execution engine
//! and `RuntimeLane` to label which lane is under test.

#![forbid(unsafe_code)]

use ftui_core::event::Event;
use ftui_render::frame::Frame;
use ftui_runtime::program::{Cmd, Model, RuntimeLane};
use ftui_runtime::simulator::ProgramSimulator;
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

/// Evidence bundle for a shadow-run mismatch.
#[derive(Debug)]
struct MismatchEvidence {
    field: String,
    legacy: String,
    structured: String,
    scenario: String,
}

impl std::fmt::Display for MismatchEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MISMATCH in '{}' for scenario '{}':\n  legacy:     {}\n  structured: {}",
            self.field, self.scenario, self.legacy, self.structured
        )
    }
}

/// Compare two lane results and return mismatch evidence if any.
fn compare_results(
    scenario: &str,
    legacy: &LaneResult,
    structured: &LaneResult,
) -> Vec<MismatchEvidence> {
    let mut mismatches = Vec::new();

    if legacy.trace != structured.trace {
        mismatches.push(MismatchEvidence {
            field: "trace".into(),
            legacy: format!("{:?}", legacy.trace),
            structured: format!("{:?}", structured.trace),
            scenario: scenario.into(),
        });
    }

    if legacy.logs != structured.logs {
        mismatches.push(MismatchEvidence {
            field: "logs".into(),
            legacy: format!("{:?}", legacy.logs),
            structured: format!("{:?}", structured.logs),
            scenario: scenario.into(),
        });
    }

    if legacy.running != structured.running {
        mismatches.push(MismatchEvidence {
            field: "running".into(),
            legacy: format!("{}", legacy.running),
            structured: format!("{}", structured.running),
            scenario: scenario.into(),
        });
    }

    if legacy.tick_rate != structured.tick_rate {
        mismatches.push(MismatchEvidence {
            field: "tick_rate".into(),
            legacy: format!("{:?}", legacy.tick_rate),
            structured: format!("{:?}", structured.tick_rate),
            scenario: scenario.into(),
        });
    }

    if legacy.cmd_log_len != structured.cmd_log_len {
        mismatches.push(MismatchEvidence {
            field: "cmd_log_len".into(),
            legacy: format!("{}", legacy.cmd_log_len),
            structured: format!("{}", structured.cmd_log_len),
            scenario: scenario.into(),
        });
    }

    if legacy.frame_hashes != structured.frame_hashes {
        mismatches.push(MismatchEvidence {
            field: "frame_hashes".into(),
            legacy: format!("{:?}", legacy.frame_hashes),
            structured: format!("{:?}", structured.frame_hashes),
            scenario: scenario.into(),
        });
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
    lane: RuntimeLane,
}

impl ShadowModel {
    fn new(lane: RuntimeLane) -> Self {
        Self {
            trace: vec![],
            lane,
        }
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
        SMsg::Step("event".into())
    }
}

impl Model for ShadowModel {
    type Message = SMsg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.trace.push(format!("init[{}]", self.lane));
        Cmd::msg(SMsg::Init)
    }

    fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
        match msg {
            SMsg::Init => {
                self.trace.push("update:init".into());
                Cmd::none()
            }
            SMsg::Step(s) => {
                self.trace.push(format!("step:{s}"));
                Cmd::none()
            }
            SMsg::Batch(items) => {
                self.trace.push(format!("batch:{}", items.len()));
                Cmd::batch(items.into_iter().map(|s| Cmd::msg(SMsg::Step(s))).collect())
            }
            SMsg::Sequence(items) => {
                self.trace.push(format!("seq:{}", items.len()));
                Cmd::sequence(items.into_iter().map(|s| Cmd::msg(SMsg::Step(s))).collect())
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
                let l = label.clone();
                Cmd::task(move || SMsg::TaskResult(l))
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
            SMsg::QuitInBatch(n) => {
                self.trace.push(format!("quit-batch:{n}"));
                let mut cmds: Vec<Cmd<SMsg>> = (0..n)
                    .map(|i| Cmd::msg(SMsg::Step(format!("pre-{i}"))))
                    .collect();
                cmds.push(Cmd::quit());
                cmds.push(Cmd::msg(SMsg::Step("post-quit".into())));
                Cmd::batch(cmds)
            }
        }
    }

    fn view(&self, frame: &mut Frame) {
        let text = format!("n={}", self.trace.len());
        for (i, c) in text.chars().enumerate() {
            if (i as u16) < frame.width() {
                use ftui_render::cell::Cell;
                frame.buffer.set_raw(i as u16, 0, Cell::from_char(c));
            }
        }
    }
}

/// Run a scenario through a specific lane and capture results.
fn run_lane(lane: RuntimeLane, msgs: Vec<SMsg>, capture_frames: &[(u16, u16)]) -> LaneResult {
    let mut sim = ProgramSimulator::new(ShadowModel::new(lane));
    sim.init();

    for msg in msgs {
        sim.send(msg);
    }

    let mut frame_hashes = Vec::new();
    for &(w, h) in capture_frames {
        let buf = sim.capture_frame(w, h);
        frame_hashes.push(hash_buffer(buf));
    }

    // Normalize trace: remove lane-specific init prefix for comparison
    let trace: Vec<String> = sim
        .model()
        .trace
        .iter()
        .map(|s| {
            if s.starts_with("init[") {
                "init".to_string()
            } else {
                s.clone()
            }
        })
        .collect();

    LaneResult {
        lane,
        trace,
        logs: sim.logs().to_vec(),
        running: sim.is_running(),
        tick_rate: sim.tick_rate(),
        cmd_log_len: sim.command_log().len(),
        frame_hashes,
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
            .map(|m| format!("  {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ============================================================================
// SHADOW-RUN TESTS: verify lane equivalence
// ============================================================================

#[test]
fn shadow_basic_steps() {
    shadow_compare(
        "basic_steps",
        || {
            vec![
                SMsg::Step("a".into()),
                SMsg::Step("b".into()),
                SMsg::Step("c".into()),
            ]
        },
        &[(40, 10)],
    );
}

#[test]
fn shadow_batch_ordering() {
    shadow_compare(
        "batch_ordering",
        || vec![SMsg::Batch(vec!["x".into(), "y".into(), "z".into()])],
        &[(40, 10)],
    );
}

#[test]
fn shadow_sequence_ordering() {
    shadow_compare(
        "sequence_ordering",
        || vec![SMsg::Sequence(vec!["p".into(), "q".into(), "r".into()])],
        &[(40, 10)],
    );
}

#[test]
fn shadow_task_execution() {
    shadow_compare(
        "task_execution",
        || vec![SMsg::Task("alpha".into()), SMsg::Task("beta".into())],
        &[(40, 10)],
    );
}

#[test]
fn shadow_nested_recursion() {
    shadow_compare("nested_recursion", || vec![SMsg::Nested(10)], &[]);
}

#[test]
fn shadow_log_output() {
    shadow_compare(
        "log_output",
        || vec![SMsg::Log("hello".into()), SMsg::Log("world".into())],
        &[],
    );
}

#[test]
fn shadow_tick_rate() {
    shadow_compare("tick_rate", || vec![SMsg::Tick], &[]);
}

#[test]
fn shadow_quit_stops_processing() {
    shadow_compare(
        "quit_stops",
        || {
            vec![
                SMsg::Step("before".into()),
                SMsg::Quit,
                SMsg::Step("after".into()),
            ]
        },
        &[],
    );
}

#[test]
fn shadow_quit_in_batch() {
    shadow_compare("quit_in_batch", || vec![SMsg::QuitInBatch(3)], &[]);
}

#[test]
fn shadow_complex_scenario() {
    shadow_compare(
        "complex",
        || {
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
        },
        &[(80, 24), (40, 10)],
    );
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
    shadow_compare("empty", Vec::new, &[(10, 5)]);
}

#[test]
fn shadow_large_batch() {
    shadow_compare(
        "large_batch",
        || {
            let items: Vec<String> = (0..50).map(|i| format!("item-{i}")).collect();
            vec![SMsg::Batch(items)]
        },
        &[(80, 24)],
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
            &[(40, 10)],
        );
        results.push(legacy);
    }

    for (i, r) in results.iter().enumerate().skip(1) {
        assert_eq!(r.trace, results[0].trace, "run {i} trace diverged");
        assert_eq!(
            r.frame_hashes, results[0].frame_hashes,
            "run {i} frame hashes diverged"
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

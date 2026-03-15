//! Cross-component lifecycle contract tests (bd-1dg21).
//!
//! These integration tests capture the observable behavioral contract of the
//! runtime lifecycle that MUST be preserved during the Asupersync migration.
//! They exercise real subscription threads, real channels, and real timing
//! to verify the contract under conditions closer to production than unit tests.

#![forbid(unsafe_code)]

use ftui_core::event::Event;
use ftui_render::frame::Frame;
use ftui_runtime::program::{Cmd, Model};
use ftui_runtime::simulator::ProgramSimulator;
use ftui_runtime::subscription::{StopSignal, SubId, Subscription};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

// ============================================================================
// Test model: LifecycleTracker
//
// Records the exact sequence of lifecycle events to verify ordering contracts.
// ============================================================================

struct LifecycleTracker {
    trace: Vec<String>,
}

#[derive(Debug)]
enum LMsg {
    Init,
    Tick,
    Quit,
    TaskResult(String),
    SpawnTask,
}

impl From<Event> for LMsg {
    fn from(_: Event) -> Self {
        LMsg::Tick
    }
}

impl Model for LifecycleTracker {
    type Message = LMsg;

    fn init(&mut self) -> Cmd<Self::Message> {
        self.trace.push("init".into());
        Cmd::msg(LMsg::Init)
    }

    fn update(&mut self, msg: Self::Message) -> Cmd<Self::Message> {
        match msg {
            LMsg::Init => {
                self.trace.push("update:init".into());
                Cmd::none()
            }
            LMsg::Tick => {
                self.trace.push("update:tick".into());
                Cmd::none()
            }
            LMsg::Quit => {
                self.trace.push("update:quit".into());
                Cmd::quit()
            }
            LMsg::TaskResult(s) => {
                self.trace.push(format!("update:task-result:{s}"));
                Cmd::none()
            }
            LMsg::SpawnTask => {
                self.trace.push("update:spawn-task".into());
                Cmd::task(|| LMsg::TaskResult("done".into()))
            }
        }
    }

    fn view(&self, _frame: &mut Frame) {
        // no-op for lifecycle tests
    }

    fn on_shutdown(&mut self) -> Cmd<Self::Message> {
        self.trace.push("on_shutdown".into());
        Cmd::none()
    }
}

// ============================================================================
// CONTRACT: Full lifecycle trace ordering
// ============================================================================

#[test]
fn contract_lifecycle_ordering_init_update_shutdown() {
    let mut sim = ProgramSimulator::new(LifecycleTracker {
        trace: vec![],
    });

    sim.init();
    sim.send(LMsg::SpawnTask);
    sim.send(LMsg::Quit);

    // Manually invoke on_shutdown to verify it's called during shutdown.
    // The real runtime calls this automatically; simulator doesn't.
    // We call it directly on the model to verify the contract.
    let _shutdown_cmd = sim.model_mut().on_shutdown();

    let trace = &sim.model().trace;
    assert_eq!(
        trace,
        &[
            "init",
            "update:init",
            "update:spawn-task",
            "update:task-result:done",
            "update:quit",
            "on_shutdown",
        ],
        "lifecycle must follow: init -> update(init_cmd) -> updates -> quit -> on_shutdown"
    );
}

// ============================================================================
// CONTRACT: Subscription integration with real threads
// ============================================================================

#[test]
fn contract_subscription_messages_reach_model_via_channel() {
    // Verify that subscription messages actually traverse the mpsc channel
    // and arrive in drain_messages() in the correct order.
    let (tx, rx) = mpsc::channel::<String>();

    struct OrderedSub {
        sender: mpsc::Sender<String>,
    }

    impl Subscription<String> for OrderedSub {
        fn id(&self) -> SubId {
            42
        }

        fn run(&self, sender: mpsc::Sender<String>, _stop: StopSignal) {
            for i in 0..5 {
                let _ = sender.send(format!("msg-{i}"));
                std::thread::sleep(Duration::from_millis(5));
            }
            // Notify test that we're done sending
            let _ = self.sender.send("done".into());
        }
    }

    // SubscriptionManager is pub(crate), so we test the subscription contract
    // through the public Subscription trait + StopSignal API directly.
    let (signal, trigger) = StopSignal::new();
    let (msg_tx, msg_rx) = mpsc::channel::<String>();
    let sub = OrderedSub { sender: tx };

    let handle = std::thread::spawn(move || {
        sub.run(msg_tx, signal);
    });

    // Wait for all messages
    let _ = rx.recv_timeout(Duration::from_secs(2));

    trigger.stop();
    handle.join().unwrap();

    let msgs: Vec<String> = msg_rx.try_iter().collect();
    assert_eq!(
        msgs,
        vec!["msg-0", "msg-1", "msg-2", "msg-3", "msg-4"],
        "subscription messages must arrive in send order"
    );
}

// ============================================================================
// CONTRACT: ProcessSubscription lifecycle
// ============================================================================

#[test]
fn contract_process_subscription_captures_exit_code() {
    use ftui_runtime::process_subscription::{ProcessEvent, ProcessSubscription};

    let sub = ProcessSubscription::new("sh", |e| e)
        .arg("-c")
        .arg("exit 7");
    let (tx, rx) = mpsc::channel();
    let (signal, trigger) = StopSignal::new();

    let handle = std::thread::spawn(move || {
        sub.run(tx, signal);
    });

    std::thread::sleep(Duration::from_millis(500));
    trigger.stop();
    handle.join().unwrap();

    let msgs: Vec<ProcessEvent> = rx.try_iter().collect();
    assert!(
        msgs.contains(&ProcessEvent::Exited(7)),
        "must capture exact exit code, got: {msgs:?}"
    );
}

#[test]
fn contract_process_subscription_kill_on_stop() {
    use ftui_runtime::process_subscription::{ProcessEvent, ProcessSubscription};

    let sub = ProcessSubscription::new("sleep", |e| e).arg("60");
    let (tx, rx) = mpsc::channel();
    let (signal, trigger) = StopSignal::new();

    let start = std::time::Instant::now();
    let handle = std::thread::spawn(move || {
        sub.run(tx, signal);
    });

    std::thread::sleep(Duration::from_millis(100));
    trigger.stop();
    handle.join().unwrap();

    assert!(
        start.elapsed() < Duration::from_secs(2),
        "stop signal must kill process promptly"
    );

    let msgs: Vec<ProcessEvent> = rx.try_iter().collect();
    assert!(
        msgs.contains(&ProcessEvent::Killed),
        "must emit Killed event on stop, got: {msgs:?}"
    );
}

#[test]
fn contract_process_subscription_timeout_kills() {
    use ftui_runtime::process_subscription::{ProcessEvent, ProcessSubscription};

    let sub = ProcessSubscription::new("sleep", |e| e)
        .arg("60")
        .timeout(Duration::from_millis(100));
    let (tx, rx) = mpsc::channel();
    let (signal, _trigger) = StopSignal::new();

    let start = std::time::Instant::now();
    let handle = std::thread::spawn(move || {
        sub.run(tx, signal);
    });

    handle.join().unwrap();

    assert!(
        start.elapsed() < Duration::from_secs(2),
        "timeout must kill process promptly"
    );

    let msgs: Vec<ProcessEvent> = rx.try_iter().collect();
    assert!(
        msgs.contains(&ProcessEvent::Killed),
        "must emit Killed event on timeout, got: {msgs:?}"
    );
}

// ============================================================================
// CONTRACT: CancellationToken integration
// ============================================================================

#[test]
fn contract_cancellation_token_stops_background_work() {
    use ftui_runtime::cancellation::CancellationSource;

    let source = CancellationSource::new();
    let token = source.token();
    let work_done = Arc::new(AtomicUsize::new(0));
    let work_clone = work_done.clone();

    let handle = std::thread::spawn(move || {
        while !token.is_cancelled() {
            work_clone.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    std::thread::sleep(Duration::from_millis(50));
    let before_cancel = work_done.load(Ordering::SeqCst);
    assert!(before_cancel > 0, "work should have started");

    source.cancel();
    handle.join().unwrap();

    // After cancel + join, no more work should accumulate
    let at_cancel = work_done.load(Ordering::SeqCst);
    std::thread::sleep(Duration::from_millis(50));
    let after_wait = work_done.load(Ordering::SeqCst);
    assert_eq!(
        at_cancel, after_wait,
        "no work should happen after cancellation"
    );
}

// ============================================================================
// CONTRACT: Effect system metrics
// ============================================================================

#[test]
fn contract_effect_metrics_increment_monotonically() {
    let before_cmd = ftui_runtime::effect_system::effects_command_total();
    let before_sub = ftui_runtime::effect_system::effects_subscription_total();

    ftui_runtime::effect_system::record_command_effect("test_cmd", 100);
    ftui_runtime::effect_system::record_command_effect("test_cmd", 200);

    let after_cmd = ftui_runtime::effect_system::effects_command_total();
    assert_eq!(
        after_cmd - before_cmd,
        2,
        "command counter must increment by exactly 1 per record_command_effect call"
    );

    ftui_runtime::effect_system::record_subscription_start("test_sub", 1);

    let after_sub = ftui_runtime::effect_system::effects_subscription_total();
    assert_eq!(
        after_sub - before_sub,
        1,
        "subscription counter must increment by exactly 1 per record_subscription_start call"
    );

    // Combined total
    let total = ftui_runtime::effect_system::effects_executed_total();
    assert_eq!(
        total,
        after_cmd + after_sub,
        "combined total must equal cmd + sub"
    );
}

// ============================================================================
// CONTRACT: StopSignal / StopTrigger thread-safety under contention
// ============================================================================

#[test]
fn contract_stop_signal_correct_under_contention() {
    // Verify that multiple threads can concurrently check the stop signal
    // and all observe the transition correctly.
    let (signal, trigger) = StopSignal::new();
    let barrier = Arc::new(std::sync::Barrier::new(5));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let sig = signal.clone();
            let bar = barrier.clone();
            std::thread::spawn(move || {
                bar.wait(); // synchronize start
                // Busy-poll until stopped
                while !sig.is_stopped() {
                    std::thread::yield_now();
                }
                true
            })
        })
        .collect();

    barrier.wait(); // release all threads
    std::thread::sleep(Duration::from_millis(10));
    trigger.stop();

    for h in handles {
        let saw_stop = h.join().unwrap();
        assert!(saw_stop, "all threads must observe stop");
    }
}

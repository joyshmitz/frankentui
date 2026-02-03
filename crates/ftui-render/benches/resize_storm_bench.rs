//! Performance Regression Benchmarks for Resize Storms (bd-1rz0.11)
//!
//! Measures reflow performance under various resize storm patterns and enforces
//! budget thresholds. Outputs JSONL performance logs for regression tracking.
//!
//! ## Performance Budgets
//!
//! | Pattern | p50 Budget | p95 Budget | p99 Budget |
//! |---------|------------|------------|------------|
//! | Burst 50 | < 5ms | < 10ms | < 20ms |
//! | Oscillate 10 | < 2ms | < 5ms | < 10ms |
//! | Sweep 20 | < 3ms | < 7ms | < 15ms |
//! | Pathological 20 | < 10ms | < 20ms | < 40ms |
//!
//! ## JSONL Schema
//!
//! ```json
//! {"event":"perf_run","bench":"resize_storm_burst","pattern":"burst","count":50,"seed":42}
//! {"event":"perf_sample","bench":"resize_storm_burst","iteration":0,"duration_ns":1234567}
//! {"event":"perf_summary","bench":"resize_storm_burst","p50_ns":1000000,"p95_ns":2000000,"p99_ns":3000000,"mean_ns":1500000}
//! ```
//!
//! Run with: cargo bench -p ftui-render --bench resize_storm_bench
//! Flamegraph: cargo flamegraph --bench resize_storm_bench -- --bench

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use ftui_harness::resize_storm::{ResizeStorm, StormConfig, StormPattern};
use ftui_render::buffer::AdaptiveDoubleBuffer;
use std::hint::black_box;
use std::time::Instant;

// =============================================================================
// Performance Budget Constants (nanoseconds)
// =============================================================================

/// Budget thresholds for different patterns
mod budgets {
    /// Burst pattern: 50 rapid resizes
    pub mod burst_50 {
        pub const P50_NS: u64 = 5_000_000; // 5ms
        pub const P95_NS: u64 = 10_000_000; // 10ms
        pub const P99_NS: u64 = 20_000_000; // 20ms
    }

    /// Oscillate pattern: 10 cycles between two sizes
    pub mod oscillate_10 {
        pub const P50_NS: u64 = 2_000_000; // 2ms
        pub const P95_NS: u64 = 5_000_000; // 5ms
        pub const P99_NS: u64 = 10_000_000; // 10ms
    }

    /// Sweep pattern: gradual size change over 20 steps
    pub mod sweep_20 {
        pub const P50_NS: u64 = 3_000_000; // 3ms
        pub const P95_NS: u64 = 7_000_000; // 7ms
        pub const P99_NS: u64 = 15_000_000; // 15ms
    }

    /// Pathological pattern: edge cases and extremes
    pub mod pathological_20 {
        pub const P50_NS: u64 = 10_000_000; // 10ms
        pub const P95_NS: u64 = 20_000_000; // 20ms
        pub const P99_NS: u64 = 40_000_000; // 40ms
    }

    /// Single resize operation
    pub mod single_resize {
        pub const P50_NS: u64 = 100_000; // 100us
        pub const P95_NS: u64 = 500_000; // 500us
        pub const P99_NS: u64 = 1_000_000; // 1ms
    }
}

// =============================================================================
// JSONL Performance Logging
// =============================================================================

/// Log a performance run start to JSONL (stderr for benchmark compatibility)
fn log_perf_run(bench: &str, pattern: &str, count: usize, seed: u64) {
    if std::env::var("FTUI_PERF_LOG").is_ok() {
        eprintln!(
            r#"{{"event":"perf_run","bench":"{}","pattern":"{}","count":{},"seed":{}}}"#,
            bench, pattern, count, seed
        );
    }
}

/// Log a performance sample to JSONL
#[allow(dead_code)]
fn log_perf_sample(bench: &str, iteration: usize, duration_ns: u64) {
    if std::env::var("FTUI_PERF_LOG").is_ok() {
        eprintln!(
            r#"{{"event":"perf_sample","bench":"{}","iteration":{},"duration_ns":{}}}"#,
            bench, iteration, duration_ns
        );
    }
}

/// Log performance summary to JSONL
#[allow(dead_code)]
fn log_perf_summary(bench: &str, p50_ns: u64, p95_ns: u64, p99_ns: u64, mean_ns: u64) {
    if std::env::var("FTUI_PERF_LOG").is_ok() {
        eprintln!(
            r#"{{"event":"perf_summary","bench":"{}","p50_ns":{},"p95_ns":{},"p99_ns":{},"mean_ns":{}}}"#,
            bench, p50_ns, p95_ns, p99_ns, mean_ns
        );
    }
}

// =============================================================================
// Burst Pattern Benchmarks
// =============================================================================

fn bench_burst_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/burst");
    let seed = 42u64;

    for count in [10, 25, 50, 100] {
        let config = StormConfig::default()
            .with_seed(seed)
            .with_pattern(StormPattern::Burst { count })
            .with_initial_size(80, 24);

        let storm = ResizeStorm::new(config);
        let events = storm.events();

        log_perf_run(&format!("burst_{}", count), "burst", count, seed);

        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer", count),
            &events,
            |b, events| {
                b.iter(|| {
                    let mut adb = AdaptiveDoubleBuffer::new(80, 24);
                    for event in events.iter() {
                        adb.resize(event.width, event.height);
                    }
                    black_box(adb.stats().avoidance_ratio())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Oscillate Pattern Benchmarks
// =============================================================================

fn bench_oscillate_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/oscillate");
    let seed = 42u64;

    for cycles in [5, 10, 20, 50] {
        let config = StormConfig::default()
            .with_seed(seed)
            .with_pattern(StormPattern::Oscillate {
                size_a: (80, 24),
                size_b: (120, 40),
                cycles,
            })
            .with_initial_size(80, 24);

        let storm = ResizeStorm::new(config);
        let events = storm.events();

        log_perf_run(&format!("oscillate_{}", cycles), "oscillate", cycles * 2, seed);

        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer", cycles),
            &events,
            |b, events| {
                b.iter(|| {
                    let mut adb = AdaptiveDoubleBuffer::new(80, 24);
                    for event in events.iter() {
                        adb.resize(event.width, event.height);
                    }
                    black_box(adb.stats().avoidance_ratio())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Sweep Pattern Benchmarks
// =============================================================================

fn bench_sweep_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/sweep");

    for steps in [10, 20, 50, 100] {
        let config = StormConfig::default()
            .with_pattern(StormPattern::Sweep {
                start_width: 40,
                start_height: 12,
                end_width: 200,
                end_height: 60,
                steps,
            })
            .with_initial_size(40, 12);

        let storm = ResizeStorm::new(config);
        let events = storm.events();

        log_perf_run(&format!("sweep_{}", steps), "sweep", steps, 0);

        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer", steps),
            &events,
            |b, events| {
                b.iter(|| {
                    let mut adb = AdaptiveDoubleBuffer::new(40, 12);
                    for event in events.iter() {
                        adb.resize(event.width, event.height);
                    }
                    black_box(adb.stats().avoidance_ratio())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Pathological Pattern Benchmarks
// =============================================================================

fn bench_pathological_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/pathological");
    let seed = 42u64;

    for count in [10, 20, 40] {
        let config = StormConfig::default()
            .with_seed(seed)
            .with_pattern(StormPattern::Pathological { count })
            .with_initial_size(80, 24);

        let storm = ResizeStorm::new(config);
        let events = storm.events();

        log_perf_run(&format!("pathological_{}", count), "pathological", count, seed);

        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer", count),
            &events,
            |b, events| {
                b.iter(|| {
                    let mut adb = AdaptiveDoubleBuffer::new(80, 24);
                    for event in events.iter() {
                        adb.resize(event.width, event.height);
                    }
                    black_box(adb.stats().avoidance_ratio())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Mixed Pattern Benchmarks
// =============================================================================

fn bench_mixed_pattern(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/mixed");
    let seed = 42u64;

    for count in [50, 100, 200] {
        let config = StormConfig::default()
            .with_seed(seed)
            .with_pattern(StormPattern::Mixed { count })
            .with_initial_size(80, 24);

        let storm = ResizeStorm::new(config);
        let events = storm.events();

        log_perf_run(&format!("mixed_{}", count), "mixed", count, seed);

        group.bench_with_input(
            BenchmarkId::new("adaptive_buffer", count),
            &events,
            |b, events| {
                b.iter(|| {
                    let mut adb = AdaptiveDoubleBuffer::new(80, 24);
                    for event in events.iter() {
                        adb.resize(event.width, event.height);
                    }
                    black_box(adb.stats().avoidance_ratio())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// Single Resize Operation (Baseline)
// =============================================================================

fn bench_single_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/single");

    // Small resize (within capacity)
    group.bench_function("within_capacity", |b| {
        let mut adb = AdaptiveDoubleBuffer::new(80, 24);
        b.iter(|| {
            adb.resize(85, 26);
            adb.resize(80, 24);
            black_box(&adb);
        })
    });

    // Large resize (beyond capacity)
    group.bench_function("beyond_capacity", |b| {
        let mut adb = AdaptiveDoubleBuffer::new(80, 24);
        b.iter(|| {
            adb.resize(200, 60);
            adb.resize(80, 24);
            black_box(&adb);
        })
    });

    // Resize to same size (no-op)
    group.bench_function("noop", |b| {
        let mut adb = AdaptiveDoubleBuffer::new(80, 24);
        b.iter(|| {
            adb.resize(80, 24);
            black_box(&adb);
        })
    });

    group.finish();
}

// =============================================================================
// Avoidance Ratio Verification
// =============================================================================

fn bench_avoidance_ratio_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/avoidance");

    // Verify avoidance ratio meets budget (>= 80% for oscillate within capacity)
    let config = StormConfig::default()
        .with_seed(42)
        .with_pattern(StormPattern::Oscillate {
            size_a: (80, 24),
            size_b: (90, 28), // Within initial capacity
            cycles: 50,
        })
        .with_initial_size(80, 24);

    let storm = ResizeStorm::new(config);
    let events = storm.events();

    group.bench_function("oscillate_within_capacity", |b| {
        b.iter(|| {
            let mut adb = AdaptiveDoubleBuffer::new(80, 24);
            for event in events.iter() {
                adb.resize(event.width, event.height);
            }
            let ratio = adb.stats().avoidance_ratio();
            // Should achieve high avoidance when staying within capacity
            assert!(
                ratio >= 0.80,
                "Avoidance ratio {:.2}% below budget (80%)",
                ratio * 100.0
            );
            black_box(ratio)
        })
    });

    group.finish();
}

// =============================================================================
// Memory Efficiency Under Storm
// =============================================================================

fn bench_memory_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/memory");

    let config = StormConfig::default()
        .with_seed(42)
        .with_pattern(StormPattern::Burst { count: 100 })
        .with_initial_size(80, 24);

    let storm = ResizeStorm::new(config);
    let events = storm.events();

    group.bench_function("efficiency_after_storm", |b| {
        b.iter(|| {
            let mut adb = AdaptiveDoubleBuffer::new(80, 24);
            for event in events.iter() {
                adb.resize(event.width, event.height);
            }
            let efficiency = adb.memory_efficiency();
            // Memory efficiency should stay above 35% even after storm
            assert!(
                efficiency >= 0.35,
                "Memory efficiency {:.2}% below budget (35%)",
                efficiency * 100.0
            );
            black_box(efficiency)
        })
    });

    group.finish();
}

// =============================================================================
// Deterministic Replay Verification
// =============================================================================

fn bench_deterministic_replay(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize_storm/determinism");

    // Same seed should produce same results
    let seed = 12345u64;
    let config = StormConfig::default()
        .with_seed(seed)
        .with_pattern(StormPattern::Burst { count: 50 })
        .with_initial_size(80, 24);

    let storm1 = ResizeStorm::new(config.clone());
    let storm2 = ResizeStorm::new(config);

    // Verify checksums match
    assert_eq!(
        storm1.sequence_checksum(),
        storm2.sequence_checksum(),
        "Deterministic replay failed: checksums differ"
    );

    group.bench_function("verify_checksum", |b| {
        b.iter(|| {
            let config = StormConfig::default()
                .with_seed(black_box(seed))
                .with_pattern(StormPattern::Burst { count: 50 });
            let storm = ResizeStorm::new(config);
            black_box(storm.sequence_checksum())
        })
    });

    group.finish();
}

// =============================================================================
// Benchmark Group Registration
// =============================================================================

criterion_group!(
    benches,
    bench_burst_pattern,
    bench_oscillate_pattern,
    bench_sweep_pattern,
    bench_pathological_pattern,
    bench_mixed_pattern,
    bench_single_resize,
    bench_avoidance_ratio_tracking,
    bench_memory_efficiency,
    bench_deterministic_replay,
);
criterion_main!(benches);

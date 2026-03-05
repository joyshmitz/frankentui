//! Profile sweep binary for flamegraph / heaptrack analysis (bd-3jlw5.7, bd-3jlw5.8).
//!
//! Renders every demo screen at 80x24 and 120x40 in a tight loop.
//! Designed to be run under `cargo flamegraph` or `heaptrack`:
//!
//!   cargo flamegraph --bin profile_sweep -p ftui-demo-showcase -- --cycles 100
//!   heaptrack cargo run --release --bin profile_sweep -p ftui-demo-showcase -- --cycles 10
//!
//! Arena comparison mode (bd-2alzw.3):
//!
//!   cargo run --release --bin profile_sweep -p ftui-demo-showcase -- --cycles 10 --arena-mode off --json
//!   cargo run --release --bin profile_sweep -p ftui-demo-showcase -- --cycles 10 --arena-mode on  --json

use std::alloc::System;
use std::time::Instant;

use ftui_core::event::Event;
use ftui_demo_showcase::app::AppModel;
use ftui_demo_showcase::screens;
use ftui_render::arena::FrameArena;
use ftui_render::frame::Frame;
use ftui_render::grapheme_pool::GraphemePool;
use ftui_runtime::{Cmd, Model};
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArenaMode {
    Off,
    On,
}

impl ArenaMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::On => "on",
        }
    }
}

#[derive(Clone, Debug)]
struct Args {
    cycles: usize,
    arena_mode: ArenaMode,
    json: bool,
}

fn usage_and_exit(message: &str) -> ! {
    eprintln!("{message}");
    eprintln!(
        "Usage: profile_sweep [--cycles N] [--arena-mode off|on] [--json]\n\
         Example: profile_sweep --cycles 10 --arena-mode on --json"
    );
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut cycles: usize = 50;
    let mut arena_mode = ArenaMode::Off;
    let mut json = false;

    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--cycles" => {
                let Some(value) = it.next() else {
                    usage_and_exit("Missing value after --cycles");
                };
                cycles = value
                    .parse()
                    .unwrap_or_else(|_| usage_and_exit("Invalid value for --cycles"));
            }
            "--arena-mode" => {
                let Some(value) = it.next() else {
                    usage_and_exit("Missing value after --arena-mode");
                };
                arena_mode = match value.as_str() {
                    "off" => ArenaMode::Off,
                    "on" => ArenaMode::On,
                    _ => usage_and_exit("Invalid value for --arena-mode (expected off|on)"),
                };
            }
            "--json" => {
                json = true;
            }
            "--help" | "-h" => {
                usage_and_exit("Profile sweep help");
            }
            other => {
                usage_and_exit(&format!("Unknown argument: {other}"));
            }
        }
    }

    Args {
        cycles,
        arena_mode,
        json,
    }
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (((sorted.len() - 1) as f64) * p).round() as usize;
    sorted[idx]
}

fn main() {
    let args = parse_args();

    let sizes: &[(u16, u16)] = &[(80, 24), (120, 40)];
    let screen_ids = screens::screen_ids();
    let total_frames = screen_ids.len() * sizes.len() * args.cycles;

    if !args.json {
        eprintln!(
            "Profile sweep: {} screens x {} sizes x {} cycles = {} renders (arena_mode={})",
            screen_ids.len(),
            sizes.len(),
            args.cycles,
            total_frames,
            args.arena_mode.as_str()
        );
    }

    let start = Instant::now();
    let mut pool = GraphemePool::new();
    let mut per_frame_us = Vec::with_capacity(total_frames);
    let mut per_frame_allocs = Vec::with_capacity(total_frames);
    let mut per_frame_alloc_bytes = Vec::with_capacity(total_frames);
    let mut total_allocs = 0usize;
    let mut total_alloc_bytes = 0usize;
    let mut total_reallocs = 0usize;
    let mut total_deallocs = 0usize;
    let mut arena = (args.arena_mode == ArenaMode::On).then(|| FrameArena::new(256 * 1024));
    let mut arena_peak_bytes = 0usize;

    for &(cols, rows) in sizes {
        let mut app = AppModel::new();
        let _: Cmd<_> = app.init();
        let _: Cmd<_> = app.update(Event::Tick.into());

        for cycle in 0..args.cycles {
            for &screen in screen_ids.iter() {
                app.current_screen = screen;
                let _: Cmd<_> = app.update(Event::Tick.into());
                let frame_start = Instant::now();
                let alloc_region = Region::new(GLOBAL);

                {
                    let mut frame = Frame::new(cols, rows, &mut pool);
                    if let Some(arena_ref) = arena.as_ref() {
                        frame.set_arena(arena_ref);
                    }
                    app.view(&mut frame);
                    // Ensure the optimizer doesn't elide the render.
                    std::hint::black_box(&frame);
                }

                let alloc_delta = alloc_region.change();
                per_frame_allocs.push(alloc_delta.allocations as u64);
                per_frame_alloc_bytes.push(alloc_delta.bytes_allocated as u64);
                total_allocs = total_allocs.saturating_add(alloc_delta.allocations);
                total_alloc_bytes = total_alloc_bytes.saturating_add(alloc_delta.bytes_allocated);
                total_reallocs = total_reallocs.saturating_add(alloc_delta.reallocations);
                total_deallocs = total_deallocs.saturating_add(alloc_delta.deallocations);

                if let Some(arena_mut) = arena.as_mut() {
                    let used = arena_mut.allocated_bytes_including_metadata();
                    arena_peak_bytes = arena_peak_bytes.max(used);
                    arena_mut.reset();
                }

                let elapsed_us = frame_start.elapsed().as_micros().min(u64::MAX as u128) as u64;
                per_frame_us.push(elapsed_us);
            }
            if !args.json && cycle % 10 == 0 {
                eprint!(".");
            }
        }
    }

    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    let renders_per_sec = if elapsed_secs > 0.0 {
        total_frames as f64 / elapsed_secs
    } else {
        0.0
    };

    let mut sorted_us = per_frame_us.clone();
    sorted_us.sort_unstable();
    let mut sorted_allocs = per_frame_allocs.clone();
    sorted_allocs.sort_unstable();
    let mut sorted_alloc_bytes = per_frame_alloc_bytes.clone();
    sorted_alloc_bytes.sort_unstable();

    let p50_us = percentile(&sorted_us, 0.50);
    let p95_us = percentile(&sorted_us, 0.95);
    let p99_us = percentile(&sorted_us, 0.99);
    let max_us = sorted_us.last().copied().unwrap_or(0);
    let alloc_p50 = percentile(&sorted_allocs, 0.50);
    let alloc_p95 = percentile(&sorted_allocs, 0.95);
    let alloc_p99 = percentile(&sorted_allocs, 0.99);
    let alloc_max = sorted_allocs.last().copied().unwrap_or(0);
    let alloc_bytes_p50 = percentile(&sorted_alloc_bytes, 0.50);
    let alloc_bytes_p95 = percentile(&sorted_alloc_bytes, 0.95);
    let alloc_bytes_p99 = percentile(&sorted_alloc_bytes, 0.99);
    let alloc_bytes_max = sorted_alloc_bytes.last().copied().unwrap_or(0);

    if args.json {
        let summary = serde_json::json!({
            "arena_mode": args.arena_mode.as_str(),
            "cycles": args.cycles,
            "screen_count": screen_ids.len(),
            "sizes": sizes.iter().map(|(w, h)| serde_json::json!({"cols": w, "rows": h})).collect::<Vec<_>>(),
            "total_frames": total_frames,
            "elapsed_ms": elapsed_secs * 1000.0,
            "renders_per_sec": renders_per_sec,
            "frame_time_us": {
                "p50": p50_us,
                "p95": p95_us,
                "p99": p99_us,
                "max": max_us
            },
            "allocations": {
                "total": total_allocs,
                "reallocations_total": total_reallocs,
                "deallocations_total": total_deallocs,
                "per_frame": {
                    "p50": alloc_p50,
                    "p95": alloc_p95,
                    "p99": alloc_p99,
                    "max": alloc_max
                }
            },
            "allocated_bytes": {
                "total": total_alloc_bytes,
                "per_frame": {
                    "p50": alloc_bytes_p50,
                    "p95": alloc_bytes_p95,
                    "p99": alloc_bytes_p99,
                    "max": alloc_bytes_max
                }
            },
            "arena_peak_bytes": arena_peak_bytes
        });
        println!("{summary}");
    } else {
        eprintln!(
            "\nDone in {:.2}s ({:.1} renders/sec) | frame_us p50={} p95={} p99={} max={} | allocs/frame p50={} p95={} p99={} max={} | arena_peak_bytes={}",
            elapsed_secs,
            renders_per_sec,
            p50_us,
            p95_us,
            p99_us,
            max_us,
            alloc_p50,
            alloc_p95,
            alloc_p99,
            alloc_max,
            arena_peak_bytes
        );
    }
}

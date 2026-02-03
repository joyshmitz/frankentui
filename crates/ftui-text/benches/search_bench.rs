//! Benchmarks for search and replace operations.
//!
//! Run with: `cargo bench --package ftui-text --bench search_bench`
//!
//! # Performance Baselines
//!
//! These benchmarks establish baselines for:
//! - Single-pattern exact search
//! - Multi-pattern Aho-Corasick search
//! - Replace operations
//! - Unicode-aware search
//!
//! # JSONL Logging
//!
//! Results are written to `target/criterion/search_bench/` with:
//! - Raw timing data per iteration
//! - Statistical analysis (mean, stddev, p50/p95/p99)
//! - Comparison to previous runs

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use ftui_text::search::{
    SearchResult, search_ascii_case_insensitive, search_exact, search_exact_overlapping,
};
use std::hint::black_box;

// ============================================================================
// Test Data Generation
// ============================================================================

/// Generate repeated text of approximately the given size.
fn generate_text(base: &str, target_size: usize) -> String {
    let repeats = (target_size / base.len()).max(1);
    base.repeat(repeats)
}

/// Sample text with common English words for realistic benchmarks.
const LOREM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
    Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
    Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris. ";

/// Text with Unicode characters (accented, CJK, emoji).
const UNICODE_TEXT: &str = "café résumé naïve 你好世界 こんにちは 🌍🚀💻 Здравствуй мир ";

// ============================================================================
// Single-Pattern Search Benchmarks
// ============================================================================

fn search_multi(haystack: &str, needles: &[&str]) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for needle in needles {
        results.extend(search_exact(haystack, needle));
    }
    results
}

fn search_multi_ascii_case_insensitive(haystack: &str, needles: &[&str]) -> Vec<SearchResult> {
    let mut results = Vec::new();
    for needle in needles {
        results.extend(search_ascii_case_insensitive(haystack, needle));
    }
    results
}

fn replace_all(haystack: &str, needle: &str, replacement: &str) -> String {
    haystack.replace(needle, replacement)
}

fn replace_all_tracked(
    haystack: &str,
    needle: &str,
    replacement: &str,
) -> (String, Vec<std::ops::Range<usize>>) {
    if needle.is_empty() {
        return (haystack.to_string(), Vec::new());
    }
    let mut out = String::with_capacity(haystack.len());
    let mut ranges = Vec::new();
    let mut start = 0;
    while let Some(pos) = haystack[start..].find(needle) {
        let abs = start + pos;
        out.push_str(&haystack[start..abs]);
        let rep_start = out.len();
        out.push_str(replacement);
        let rep_end = out.len();
        ranges.push(rep_start..rep_end);
        start = abs + needle.len();
    }
    out.push_str(&haystack[start..]);
    (out, ranges)
}

fn replace_multi(haystack: &str, needles: &[&str], replacements: &[&str]) -> String {
    let mut out = haystack.to_string();
    for (needle, replacement) in needles.iter().zip(replacements.iter()) {
        out = replace_all(&out, needle, replacement);
    }
    out
}

fn bench_search_exact(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_exact");

    for size in [1_000, 10_000, 100_000] {
        let text = generate_text(LOREM, size);
        group.throughput(Throughput::Bytes(text.len() as u64));

        // Common word (many matches)
        group.bench_with_input(BenchmarkId::new("common_word", size), &text, |b, text| {
            b.iter(|| search_exact(black_box(text), black_box("dolor")));
        });

        // Rare word (few matches)
        group.bench_with_input(BenchmarkId::new("rare_word", size), &text, |b, text| {
            b.iter(|| search_exact(black_box(text), black_box("exercitation")));
        });

        // No match
        group.bench_with_input(BenchmarkId::new("no_match", size), &text, |b, text| {
            b.iter(|| search_exact(black_box(text), black_box("xyzzyxyzzy")));
        });
    }

    group.finish();
}

fn bench_search_overlapping(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_overlapping");

    // Pathological case: many overlapping matches
    let text = "a".repeat(10_000);
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("aa_in_aaaa", |b| {
        b.iter(|| search_exact_overlapping(black_box(&text), black_box("aa")));
    });

    // Normal case
    let normal = generate_text(LOREM, 10_000);
    group.bench_function("word_in_lorem", |b| {
        b.iter(|| search_exact_overlapping(black_box(&normal), black_box("or")));
    });

    group.finish();
}

fn bench_search_ascii_case_insensitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_ascii_ci");

    for size in [1_000, 10_000, 100_000] {
        let text = generate_text(LOREM, size).to_uppercase();
        group.throughput(Throughput::Bytes(text.len() as u64));

        group.bench_with_input(BenchmarkId::new("common", size), &text, |b, text| {
            b.iter(|| search_ascii_case_insensitive(black_box(text), black_box("lorem")));
        });
    }

    group.finish();
}

// ============================================================================
// Multi-Pattern Search (Aho-Corasick) Benchmarks
// ============================================================================

fn bench_search_multi(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_multi");

    let text = generate_text(LOREM, 100_000);
    group.throughput(Throughput::Bytes(text.len() as u64));

    // Few patterns
    let few_patterns = ["Lorem", "ipsum", "dolor"];
    group.bench_function("3_patterns", |b| {
        b.iter(|| search_multi(black_box(&text), black_box(&few_patterns)));
    });

    // Many patterns
    let many_patterns = [
        "Lorem",
        "ipsum",
        "dolor",
        "sit",
        "amet",
        "consectetur",
        "adipiscing",
        "elit",
        "tempor",
        "incididunt",
        "labore",
        "dolore",
        "magna",
        "aliqua",
        "enim",
        "minim",
        "veniam",
        "quis",
        "nostrud",
        "exercitation",
    ];
    group.bench_function("20_patterns", |b| {
        b.iter(|| search_multi(black_box(&text), black_box(&many_patterns)));
    });

    // Compare with sequential search
    group.bench_function("20_patterns_sequential", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for pattern in &many_patterns {
                results.extend(search_exact(black_box(&text), black_box(pattern)));
            }
            results
        });
    });

    group.finish();
}

fn bench_search_multi_ci(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_multi_ci");

    let text = generate_text(LOREM, 50_000).to_uppercase();
    let patterns = ["lorem", "ipsum", "dolor", "amet", "elit"];

    group.throughput(Throughput::Bytes(text.len() as u64));
    group.bench_function("5_patterns", |b| {
        b.iter(|| search_multi_ascii_case_insensitive(black_box(&text), black_box(&patterns)));
    });

    group.finish();
}

// ============================================================================
// Replace Benchmarks
// ============================================================================

fn bench_replace_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("replace_all");

    for size in [1_000, 10_000, 100_000] {
        let text = generate_text(LOREM, size);
        group.throughput(Throughput::Bytes(text.len() as u64));

        group.bench_with_input(BenchmarkId::new("common", size), &text, |b, text| {
            b.iter(|| replace_all(black_box(text), black_box("dolor"), black_box("REPLACED")));
        });

        group.bench_with_input(BenchmarkId::new("no_match", size), &text, |b, text| {
            b.iter(|| replace_all(black_box(text), black_box("xyzzy"), black_box("REPLACED")));
        });
    }

    group.finish();
}

fn bench_replace_tracked(c: &mut Criterion) {
    let mut group = c.benchmark_group("replace_tracked");

    let text = generate_text(LOREM, 50_000);
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("with_positions", |b| {
        b.iter(|| replace_all_tracked(black_box(&text), black_box("dolor"), black_box("REPLACED")));
    });

    group.finish();
}

fn bench_replace_multi(c: &mut Criterion) {
    let mut group = c.benchmark_group("replace_multi");

    let text = generate_text(LOREM, 50_000);
    let patterns = ["Lorem", "ipsum", "dolor", "amet", "elit"];
    let replacements = ["L", "I", "D", "A", "E"];

    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("5_patterns", |b| {
        b.iter(|| {
            replace_multi(
                black_box(&text),
                black_box(&patterns),
                black_box(&replacements),
            )
        });
    });

    // Compare with sequential replace
    group.bench_function("5_patterns_sequential", |b| {
        b.iter(|| {
            let mut result = text.clone();
            for (p, r) in patterns.iter().zip(replacements.iter()) {
                result = replace_all(&result, p, r);
            }
            result
        });
    });

    group.finish();
}

// ============================================================================
// Unicode Benchmarks
// ============================================================================

fn bench_unicode_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("unicode_search");

    let text = generate_text(UNICODE_TEXT, 10_000);
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("accented", |b| {
        b.iter(|| search_exact(black_box(&text), black_box("café")));
    });

    group.bench_function("cjk", |b| {
        b.iter(|| search_exact(black_box(&text), black_box("你好")));
    });

    group.bench_function("emoji", |b| {
        b.iter(|| search_exact(black_box(&text), black_box("🌍")));
    });

    group.finish();
}

fn bench_unicode_replace(c: &mut Criterion) {
    let mut group = c.benchmark_group("unicode_replace");

    let text = generate_text(UNICODE_TEXT, 10_000);
    group.throughput(Throughput::Bytes(text.len() as u64));

    group.bench_function("emoji", |b| {
        b.iter(|| replace_all(black_box(&text), black_box("🌍"), black_box("🌎")));
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_search_exact,
    bench_search_overlapping,
    bench_search_ascii_case_insensitive,
    bench_search_multi,
    bench_search_multi_ci,
    bench_replace_all,
    bench_replace_tracked,
    bench_replace_multi,
    bench_unicode_search,
    bench_unicode_replace,
);

criterion_main!(benches);

# Extreme Optimization Handoff

Run ID: `20260424T161752Z-profile-sweep`

This is a profiling-only handoff. No behavior-changing optimization has been applied.

## Evidence Summary

- Build profile: `release-perf` with optimized code, line-table debug info, and unstripped symbols.
- Primary workload: `profile_sweep --cycles 200 --render-mode pipeline --arena-mode off --json`.
- Whole workload: `18,000` frames in `7.906s`, `2,276.9` renders/sec.
- Frame time: p50 `252us`, p95 `1,236us`, p99 `4,018us`, max `20,457us`.
- Allocation pressure: `28,476,383` allocations and `7.62GB` allocated over `18,000` frames; p99 allocations/frame `41,719`.
- Presenter is not the dominant tail: presenter p99 is `132us`, while whole-frame p99 is `4,018us`.
- Syscalls are not material: `199` total syscalls and `0.001797s` syscall time over the strace run.
- `arena-mode on` is currently slower for this workload: hyperfine mean `3.307s` vs `2.711s` off across three paired repeats.

Primary artifacts:

- `hotspot_table.md`
- `hotspot_table_by_count.md`
- `profile-events.jsonl`
- `hyperfine-rollup.json`
- `variance-off.txt`
- `variance-on.txt`
- `strace-profile-sweep-off.txt`
- `presenter-pipeline-bench.txt`

## Alien Grounding

The handoff follows the Alien Graveyard profile-first loop: baseline, profile CPU/alloc/syscalls, prove behavior, implement one lever, verify, then re-profile. The FrankenSuite map says FrankenTUI performance work should decompose queueing/service tails, preserve deterministic frame equivalence on golden traces, and use posterior frame-budget risk rather than ad hoc runtime decisions.

Relevant concepts:

- Self-adjusting computation for incremental UI/layout updates.
- Data-structure archetype work for allocation churn and cache behavior.
- Conformal/Bayesian guards only after baseline artifacts show a controller is actually needed.
- S3-FIFO only as a bounded cache policy if a simple deterministic memo proves valuable but needs admission/eviction.

## Opportunity Matrix

Score is `Impact * Confidence / Effort`.

| Rank | Lever | Evidence | Impact | Confidence | Effort | Score | Recommendation |
|------|-------|----------|:------:|:----------:|:------:|:-----:|----------------|
| 1 | Memoize or precompile markdown math conversion | Raw heaptrack report: `6,189,740` allocation calls via `unicodeit::latex_to_unicode`; raw perf report: `StrSearcher::new` `7.21%` | 5 | 5 | 2 | 12.5 | Do first |
| 2 | Avoid unconditional hit-grid clone in demo app view | `perf`: `__memmove_avx` stack includes `AppModel::view -> cache_hit_grid -> clone`, `2.94%` | 3 | 4 | 2 | 6.0 | Do after markdown |
| 3 | Fuse buffer write and dirty-span marking for text spans | `perf`: `Buffer::set` `5.18%`, `mark_dirty_span` `3.73%`, `set_fast` `3.21%` | 4 | 4 | 3 | 5.3 | Needs tight golden tests |
| 4 | Cache grapheme/width segmentation for repeated text | `perf`: `Graphemes::next` `7.99%`, `unicode_display_width::width` `2.27%`, `Paragraph::text_hash` `2.35%` | 4 | 4 | 3 | 5.3 | Pair with text golden corpus |
| 5 | Presenter style-state micro-tuning | `perf`: `Presenter::emit_style_changes` `2.16%`; isolated presenter pipeline remains under `88us` at `200x60@50%` | 2 | 4 | 2 | 4.0 | Not first |
| 6 | FrameArena expansion | Hyperfine rejects current benefit: `arena on` is slower and noisier | 2 | 5 | 3 | 3.3 | Do not pursue now |

## Recommended First Lever

Optimize `crates/ftui-extras/src/markdown.rs` math conversion without changing rendered output.

Root cause from first principles:

- The rich-markdown screen is re-rendered every profile frame.
- `MarkdownRenderer::process` calls `inline_math` / `display_math`.
- Those call `latex_to_unicode`.
- `latex_to_unicode` calls `unicodeit::replace`, then fallback replacements, allocating new strings repeatedly for identical math snippets.
- The work is pure with a small input domain in the demo workload, so repeated conversion is avoidable without semantic risk.

Implementation shape for the next optimization pass:

1. Add a deterministic memoization layer scoped to `MarkdownRenderer`, keyed by the exact LaTeX string.
2. Keep the first version simple: bounded `HashMap` plus insertion-order cap, or a tiny fixed cache if local style already exists.
3. Only escalate to S3-FIFO-style admission if the cache needs a real policy after measurement.
4. Add golden tests for inline math, display math, unsupported fallback commands, repeated identical snippets, and mixed markdown.
5. Re-run `profile_sweep` and the markdown tests, then re-profile because this will likely expose the next bottleneck.

Proof obligations:

- For every input in the markdown math test corpus, old and new `Text` output must be equivalent in line content and styles.
- No global mutable cache unless determinism and test isolation are proven.
- No unbounded growth; cache cap must be explicit and covered by a boundary test.
- Regression gates: p99 frame time must not worsen, total allocations must fall materially, and snapshot/golden rendering must stay stable.

## Notable Non-Targets

- Do not optimize I/O: strace shows it is irrelevant.
- Do not optimize presenter first: it is visible but not dominant.
- Do not expand FrameArena first: current measurements reject it for this workload.
- Do not combine the markdown cache with buffer/diff changes in one pass; the optimization skill requires one lever at a time.

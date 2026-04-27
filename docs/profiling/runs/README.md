# Profiling Runs

This directory keeps compact, reviewable profiling evidence from past optimization
passes.

Raw profiler captures, generated heaptrack reports, exported Samply JSON,
Criterion/RCH console logs, and stdout/stderr logs are intentionally not tracked
here. Recreate those artifacts locally under `tests/artifacts/perf/` when a new
profiling pass needs raw data.

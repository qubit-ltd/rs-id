# Qubit Snowflake throughput benchmark design

## Goal

Add a reproducible, repository-owned benchmark for the current Qubit
Snowflake generator and preserve the July 15, 2026 baseline results in an
English Markdown report. The benchmark must live outside `src/` and `tests/`.

## Repository layout

- `benches/qubit_snowflake_throughput.rs` is a standalone Cargo benchmark
  target with `harness = false`.
- `docs/benchmarks/qubit-snowflake-throughput.md` records the environment,
  method, commands, measurements, limitations, and conclusions in English.
- `Cargo.toml` declares the benchmark target and includes both benchmark code
  and benchmark documentation in published packages.

The benchmark uses only the standard library and this crate. It does not add
Criterion or another development dependency because the workload measures
capacity across complete clock slices rather than nanosecond-scale operation
latency.

## Measurement method

The executable benchmarks the public `QubitSnowflakeGenerator` API with a real
`SystemTime` clock and one generator shared by all worker threads. It covers
both millisecond and second timestamp precision and runs with 1, 2, 4, and 6
worker threads.

Each case excludes generator construction and startup-fence latency from the
throughput interval. Measurement begins on a fresh clock slice and counts IDs
whose decoded timestamps fall within a fixed number of complete slices. This
avoids the inflated result produced when a short wall-clock interval consumes
sequence capacity from parts of two boundary slices.

The default duration is approximately five seconds per case. Output includes
the precision, thread count, measured slice count, generated ID count,
theoretical capacity, capacity utilization, elapsed seconds, and IDs per
second. Any generator error terminates the benchmark with a descriptive
message.

## Benchmark report

The report records the exact hardware and Rust toolchain used for the captured
baseline, the release build settings, the invocation command, results for all
cases, theoretical limits from the current layout, and interpretation of
thread scaling. It explicitly states that the numbers characterize one
machine and are not portable guarantees.

The recorded baseline is regenerated with the repository benchmark after the
benchmark target is added. Results are copied verbatim into the report rather
than inferred from the earlier temporary binaries.

## Verification

Verification consists of:

1. `./align-ci.sh`
2. `cargo test --all-features`
3. `cargo package --allow-dirty --list`
4. `RUSTFLAGS="-C target-cpu=native" cargo bench --bench qubit_snowflake_throughput`

The benchmark is successful when all eight precision/thread cases complete,
their generated counts do not exceed their theoretical capacities, and the
English report matches the fresh command output.

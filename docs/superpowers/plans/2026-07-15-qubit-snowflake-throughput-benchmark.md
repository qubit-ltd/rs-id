# Qubit Snowflake Throughput Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a reproducible Cargo benchmark for Qubit Snowflake throughput and publish a fresh English baseline report.

**Architecture:** A standalone `harness = false` target in `benches/` drives one shared public generator with 1, 2, 4, or 6 worker threads. Workers generate batches of 64 IDs and decode only the final boundary batch, preserving complete-slice accounting without adding per-ID decoding to the hot path.

**Tech Stack:** Rust 1.94, Cargo native benchmark targets, standard-library threads and synchronization, and the public `qubit-id` API.

## Global Constraints

- Keep benchmark code outside `src/` and `tests/`.
- Add no dependencies.
- Cover millisecond and second precision with 1, 2, 4, and 6 workers.
- Use a real `SystemTime` clock and one generator shared by all workers.
- Measure approximately five complete seconds per case and exclude startup-fence latency.
- Write the result report in English and describe the values as a machine-specific baseline rather than a guarantee.
- Do not commit or push unless the user separately authorizes it.

## File map

- Create `benches/qubit_snowflake_throughput.rs`: alignment, worker coordination, capacity accounting, and output.
- Create `docs/benchmarks/qubit-snowflake-throughput.md`: environment, method, command, fresh results, interpretation, and limitations.
- Modify `Cargo.toml`: package the benchmark assets and declare the custom benchmark target.

---

### Task 1: Register and implement the benchmark

**Files:**
- Modify: `Cargo.toml`
- Create: `benches/qubit_snowflake_throughput.rs`

**Interfaces:**
- Consumes: `QubitSnowflakeGenerator::with_options`, `IdGenerator::next_id`, `QubitSnowflakeLayout::decode`, `QubitSnowflakeParts::timestamp`, `TimestampPrecision::sequence_bits`, and `TimestampPrecision::divisor_millis`.
- Produces: Cargo target `qubit_snowflake_throughput`, printing one line per precision/thread case.

- [ ] **Step 1: Register the missing target**

Add `/benches/**` and `/docs/benchmarks/**` to the package `include` list, then append:

```toml
[[bench]]
name = "qubit_snowflake_throughput"
harness = false
```

- [ ] **Step 2: Verify the source is required**

Run `cargo check --bench qubit_snowflake_throughput`.

Expected: failure stating that `benches/qubit_snowflake_throughput.rs` does not exist.

- [ ] **Step 3: Create constants, imports, and the entry point**

Create `benches/qubit_snowflake_throughput.rs` with the project copyright header, module documentation, standard-library synchronization/time imports, and these public crate imports:

```rust
use qubit_id::{
    IdGenerator, IdMode, QubitSnowflakeGenerator, QubitSnowflakeLayout,
    TimestampPrecision,
};

const HOST: u64 = 0;
const WORKER_COUNTS: [usize; 4] = [1, 2, 4, 6];
const MILLIS_SLICES: u64 = 5_000;
const SECOND_SLICES: u64 = 5;
const BATCH_SIZE: usize = 64;
```

Implement `main()` as nested iteration over both precision variants and `WORKER_COUNTS`, calling `run_case(precision, worker_count)` for all eight combinations.

- [ ] **Step 4: Implement clock helpers**

Add documented private functions with these exact signatures:

```rust
fn current_timestamp(epoch: SystemTime, precision: TimestampPrecision) -> u64
fn wait_for_fresh_slice(epoch: SystemTime, precision: TimestampPrecision) -> u64
fn slice_count(precision: TimestampPrecision) -> u64
fn precision_name(precision: TimestampPrecision) -> &'static str
```

`current_timestamp` computes milliseconds since `epoch`, divides by `precision.divisor_millis()`, and converts to `u64` with a descriptive `expect`. `wait_for_fresh_slice` sleeps 50 microseconds per poll for millisecond precision or 1 millisecond for second precision and returns the first timestamp greater than the initial value. The other functions return 5,000/5 and `millisecond`/`second` respectively.

- [ ] **Step 5: Implement the worker hot path**

Add this documented signature:

```rust
fn generate_until_target(
    generator: &QubitSnowflakeGenerator,
    epoch: SystemTime,
    precision: TimestampPrecision,
    start_timestamp: u64,
    slice_count: u64,
) -> u64
```

Set `target_timestamp = start_timestamp + slice_count`. Repeatedly fill `[0_u64; BATCH_SIZE]` through `next_id()`. When the current timestamp remains before the target, count the whole batch. At or after the boundary, decode only the final batch and count IDs whose timestamp is in `start_timestamp..target_timestamp`, then return. Every `expect` message must identify the failed operation.

- [ ] **Step 6: Implement coordination and output**

Implement documented `fn run_case(precision: TimestampPrecision, worker_count: usize)`. Construct a sequential generator using host 0 and `UNIX_EPOCH`, then call `next_id()` once before timing to complete the startup fence. Spawn workers around `Barrier::new(worker_count + 1)` using one `Arc<QubitSnowflakeGenerator>` and an `Arc<AtomicU64>` start timestamp.

The main thread aligns with `wait_for_fresh_slice`, stores the timestamp with release ordering, starts `Instant`, and releases the barrier. Workers load it with acquire ordering and call `generate_until_target`. Join counts and calculate:

```rust
let capacity = slice_count * (1_u64 << precision.sequence_bits());
let utilization = generated as f64 * 100.0 / capacity as f64;
let throughput = generated as f64 / elapsed.as_secs_f64();
```

Print precision, threads, slices, start timestamp, target timestamp, count, capacity, utilization, elapsed seconds, and IDs/s on one line.

- [ ] **Step 7: Format and compile**

Run `./align-ci.sh` and `cargo check --bench qubit_snowflake_throughput`.

Expected: both commands exit 0 with no compiler errors.

### Task 2: Capture and document the baseline

**Files:**
- Create: `docs/benchmarks/qubit-snowflake-throughput.md`

**Interfaces:**
- Consumes: eight benchmark output lines plus local CPU, toolchain, date, and revision metadata.
- Produces: an English report whose numeric rows exactly match the fresh run.

- [ ] **Step 1: Capture metadata**

Run:

```bash
rustc --version --verbose
cargo --version
lscpu
git rev-parse HEAD
```

Record the date as 2026-07-15, exact commit, CPU model and topology, Rust version, Cargo version, Cargo `bench` profile, and `-C target-cpu=native` flags.

- [ ] **Step 2: Capture all eight cases**

Run:

```bash
RUSTFLAGS="-C target-cpu=native" \
    cargo bench --bench qubit_snowflake_throughput \
    | tee /tmp/qubit-snowflake-throughput.txt
```

Expected: eight lines, ordered as millisecond 1/2/4/6 then second 1/2/4/6. Every count is at or below capacity.

- [ ] **Step 3: Write the English report**

Create `docs/benchmarks/qubit-snowflake-throughput.md` with these exact sections:

1. `# Qubit Snowflake Throughput Benchmark`
2. `## Scope`: one-machine baseline, not a portable guarantee.
3. `## Environment`: date, revision, CPU, Rust, Cargo, build profile, and Rust flags.
4. `## Method`: real clock, sequential mode, one shared generator, startup excluded, 5,000 millisecond slices or 5 second slices, batch size 64, boundary-only decoding, and capacities of 4,096 IDs/ms and 4,194,304 IDs/s.
5. `## Command`: the exact `RUSTFLAGS` command above.
6. `## Results`: an eight-row table containing precision, threads, IDs, capacity, utilization, elapsed seconds, and IDs/s copied from `/tmp/qubit-snowflake-throughput.txt`.
7. `## Interpretation`: identify the best millisecond worker count; explain mutex contention and second-mode sequence saturation; state that removing one sequence bit halves limits to 2,048,000 IDs/s and 2,097,152 IDs/s.
8. `## Limitations`: public-API overhead, scheduling/frequency/thermal variance, one shared generator, and excluded startup latency.

Do not retain substitution markers or reuse temporary-binary results when the fresh repository run differs.

- [ ] **Step 4: Verify arithmetic and transcription**

Compare all eight report rows to `/tmp/qubit-snowflake-throughput.txt`. Recalculate utilization as `count / capacity * 100`, preserve elapsed and throughput precision from stdout, and confirm the documented command equals the command run.

Expected: eight matching rows, correct percentages, no count above capacity, and no placeholder markers.

### Task 3: Verify the repository change

**Files:**
- Verify: `Cargo.toml`
- Verify: `benches/qubit_snowflake_throughput.rs`
- Verify: `docs/benchmarks/qubit-snowflake-throughput.md`

**Interfaces:**
- Consumes: completed target and report.
- Produces: fresh formatting, lint, test, package-content, benchmark, and diff evidence.

- [ ] **Step 1: Apply and verify project formatting and lints**

Run `./align-ci.sh`.

Expected: the project-selected rustfmt and clippy toolchains finish with no warnings.

- [ ] **Step 2: Verify tests**

Run `cargo test --all-features`.

Expected: all unit, integration, and documentation tests pass with zero failures.

- [ ] **Step 3: Verify package contents**

Run `cargo package --allow-dirty --list` because this workflow intentionally verifies before committing.

Expected: both benchmark source and English report are listed.

- [ ] **Step 4: Re-run the published command**

Run `RUSTFLAGS="-C target-cpu=native" cargo bench --bench qubit_snowflake_throughput`.

Expected: all eight cases complete and no generated count exceeds capacity. Results may vary from the recorded baseline.

- [ ] **Step 5: Inspect only the intended patch**

Run `git --no-pager diff --check`, `git status --short`, and a path-scoped `git --no-pager diff` for `Cargo.toml`, `benches/`, `docs/benchmarks/`, and the benchmark design/plan documents.

Expected: no whitespace errors. Leave the already-observed `.rs-ci` submodule pointer difference unchanged.

# Qubit Snowflake Throughput Benchmark

## Scope

This document records a single-machine sustained-throughput and startup-latency
baseline for `SnowflakeGenerator`. It is a reproducible observation on the
environment below, not a performance guarantee for other hardware, operating
systems, Rust versions, or workloads.

## Environment

- Date: 2026-07-21
- Branch: `dev-starfish`
- Repository base revision: `4faa20cd673eea3ece3773b7afff2e68a50fa03d`
- Working tree: base revision plus the `compose_at` API rename, string-adapter
  dispatch cases, async timer-completion error propagation, tests, and
  documentation changes described in this report
- Operating system: Linux 6.17.0-35-generic, x86-64
- CPU: Intel Core i5-9600K at 3.70 GHz, up to 4.60 GHz
- Topology: 1 socket, 6 physical cores, 6 logical CPUs, no SMT
- Rust: 1.94.0 (`4a4ef493e 2026-03-02`)
- Cargo: 1.94.0 (`85eff7c80 2026-01-15`)
- Build profile: Cargo `bench` (optimized)
- Rust flags: `-C target-cpu=native`

## Method

The benchmark uses the real system clock and one sequential-mode Qubit
Snowflake generator shared by all worker threads. It obtains the default epoch
from the generator instead of substituting a benchmark-only epoch. Benchmark
generators explicitly use `RestartPolicy::Immediate`, matching the current
builder default. Each throughput sample constructs a fresh generator, generates
100,000 untimed warm-up IDs,
aligns to a fresh clock slice, and then measures 2,000 millisecond slices or 2
second slices. Every precision and worker-count combination is sampled three
times.

Workers generate IDs in batches of 64. Normal batches remain on the generation
hot path; only the final boundary batch is decoded so IDs outside the measured
timestamp range are not counted. Each sample asserts that its count does not
exceed the layout capacity: 4,096 IDs per millisecond slice or 4,194,304 IDs
per second slice. The tables report the sample with the median throughput, plus
the minimum and maximum throughput across all three samples.

Startup latency is measured separately over 10,000 fresh instances with
`RestartPolicy::Immediate`. Each observation includes builder construction and
the first `generate` call, measuring the default immediate-allocation path. It
is excluded from sustained throughput timing.

Before any measurement, the executable performs a deterministic startup
configuration check for both timestamp precisions. It verifies that the shared
startup builder produces sequence zero immediately and that explicitly
overriding it with `WaitNextSlice` returns `RetryAfter`. This check uses a fixed
wall clock and `try_generate`; it does not wait, sleep, or contribute samples.

Before sustained-throughput sampling, the benchmark also runs 200,000-operation
fixed workloads through concrete and
`Arc<dyn IdGenerator<Id>>`
synchronous paths, plus concrete and
`Arc<dyn AsyncIdGenerator<Id>>`
asynchronous paths. Both numeric and `Id::to_string()` paths are included.
The asynchronous `bridge_per_call` cases call `block_on` once per ID, so every
operation includes entering and leaving the current-thread executor. The
`async_task` cases enter it once, then await all measured allocations inside one
task; their timed loop excludes per-ID executor entry and exit. Both boundaries
use the same warm-up and measured operation counts. These are dispatch and
string-conversion observations, not capacity measurements, because they can
cross clock slices.

## Command

```bash
RUSTFLAGS="-C target-cpu=native" \
    cargo bench --bench qubit_snowflake_throughput
```

Run only the startup configuration check with:

```bash
cargo bench --locked --bench qubit_snowflake_throughput -- --check-startup
```

Execute every dispatch case with a short sample and verify its completion count:

```bash
cargo bench --locked --bench qubit_snowflake_throughput -- --check-dispatch
```

The startup short path prints `startup-check passed` and exits before dispatch,
throughput, or startup latency measurements. The dispatch short path executes
all sync, `bridge_per_call`, and `async_task` cases with 128 measured operations
and 16 warm-up operations, prints the completed count for each case, then exits
before sustained throughput or startup latency measurements. Normal benchmark
runs use the same dispatch cases with the full sample size.

The run reported this configuration:

```text
configuration throughput_samples=3 startup_samples=10000 warm_up_ids=100000
```

## Dispatch Results

| Path | Iterations | Elapsed | Operations/s |
|---|---:|---:|---:|
| Sync numeric, concrete | 200,000 | 0.014224 s | 14,060,653 |
| Sync numeric, `Arc<dyn ...>` | 200,000 | 0.013786 s | 14,507,162 |
| Sync string, concrete | 200,000 | 0.020082 s | 9,959,115 |
| Sync string, `Arc<dyn ...>` | 200,000 | 0.018652 s | 10,722,475 |
| Async `bridge_per_call`, concrete numeric | 200,000 | 0.047779 s | 4,185,951 |
| Async `bridge_per_call`, dyn numeric | 200,000 | 0.049252 s | 4,060,779 |
| Async `bridge_per_call`, concrete string | 200,000 | 0.055613 s | 3,596,313 |
| Async `bridge_per_call`, dyn string | 200,000 | 0.058587 s | 3,413,744 |

The four async values above are historical `bridge_per_call` measurements from
the environment and date recorded at the top of this report. They do not
measure the single-task await boundary. Results for the new `async_task` cases
are recorded below from three fresh default benchmark runs. Do not compare the
two boundaries as an isolated dynamic-dispatch experiment because executor
entry and exit costs differ.

### Async task measurements

Measured on 2026-10-30 with Linux 7.0.0-38-generic, x86-64, Intel Core
i5-9600K (6 physical cores, 6 logical CPUs), rustc/cargo 1.94.0, optimized
Cargo bench profile, and `RUSTFLAGS="-C target-cpu=native"`. Each run used 20,000
warm-up allocations and 200,000 measured allocations per case. These runs shared
the workspace machine with other build and validation work, so the values are
observations of that environment rather than isolated hardware measurements.

Raw per-run results:

| Case | Run 1 elapsed / ops/s | Run 2 elapsed / ops/s | Run 3 elapsed / ops/s |
|---|---:|---:|---:|
| Concrete numeric | 0.113142 s / 1,767,694 | 0.019058 s / 10,494,386 | 0.061230 s / 3,266,377 |
| Dyn numeric | 0.074723 s / 2,676,540 | 0.026966 s / 7,416,845 | 0.071422 s / 2,800,271 |
| Concrete string | 0.134858 s / 1,483,038 | 0.024540 s / 8,150,086 | 0.065621 s / 3,047,786 |
| Dyn string | 0.090836 s / 2,201,774 | 0.040013 s / 4,998,433 | 0.062059 s / 3,222,741 |

Across those three runs:

| Case | Minimum ops/s | Median ops/s | Maximum ops/s |
|---|---:|---:|---:|
| Concrete numeric | 1,767,694 | 3,266,377 | 10,494,386 |
| Dyn numeric | 2,676,540 | 2,800,271 | 7,416,845 |
| Concrete string | 1,483,038 | 3,047,786 | 8,150,086 |
| Dyn string | 2,201,774 | 3,222,741 | 4,998,433 |

All 12 dispatch cases (four sync and eight async boundary/type combinations)
reported `completed=200000` in every full run, and each run exited successfully.
The three-run spread is wide, especially for numeric cases; these data do not
support a general concrete versus dynamic dispatch performance claim.

### Provenance of the October measurements

The exact source revision and working-tree differences for the 2026-10-30
measurements were not recorded in this report and cannot be recovered from
the measurement information currently available. The July base revision
listed in Environment does not identify the source used for these October
runs. The current HEAD is not evidence of the measured revision.

The report entry was added in documentation record revision
`41a257d670815d066ef3e1e1182aa02e1ce8deed`; this identifies the report change,
not the source revision used for the measurements.

The tables above retain the recorded per-run results. A separate original
measurement log is not available in the retained material; these tables
must not be treated as a replacement for that log. The missing source
snapshot limits exact reproduction of this historical batch. No new
measurements were substituted for the recorded results.

## Sustained Throughput Results

| Precision | Threads | Samples | Slices/sample | Capacity/sample | Median IDs | Median utilization | Median elapsed | Min IDs/s | Median IDs/s | Max IDs/s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Millisecond | 1 | 3 | 2,000 | 8,192,000 | 8,190,869 | 99.99% | 2.000215 s | 4,087,022 | 4,094,994 | 4,095,602 |
| Millisecond | 2 | 3 | 2,000 | 8,192,000 | 8,192,000 | 100.00% | 2.000170 s | 4,095,037 | 4,095,652 | 4,095,671 |
| Millisecond | 4 | 3 | 2,000 | 8,192,000 | 8,188,732 | 99.96% | 2.000137 s | 4,092,081 | 4,094,085 | 4,095,014 |
| Millisecond | 6 | 3 | 2,000 | 8,192,000 | 8,125,906 | 99.19% | 2.000151 s | 4,008,809 | 4,062,647 | 4,093,753 |
| Second | 1 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.000193 s | 4,193,851 | 4,193,900 | 4,194,749 |
| Second | 2 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 1.999885 s | 4,187,942 | 4,194,544 | 4,195,791 |
| Second | 4 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.000324 s | 4,192,767 | 4,193,624 | 4,194,464 |
| Second | 6 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.000489 s | 4,193,082 | 4,193,279 | 4,194,924 |

## Startup Latency Results

| Precision | Samples | Minimum | Median | Maximum |
|---|---:|---:|---:|---:|
| Millisecond | 10,000 | 860 ns | 880 ns | 24,509 ns |
| Second | 10,000 | 862 ns | 880 ns | 14,823 ns |

## Interpretation

Millisecond mode reached approximately 4.095 million IDs/s with one to four
workers. The six-worker median fell to 4.063 million IDs/s and showed the widest
range, while its maximum still approached the 4,096 IDs/ms layout limit. More
callers cannot raise that hard limit and still contend for the same generator
mutex.

Second mode is also sequence-capacity-bound. Every sample consumed the complete
22-bit sequence space for both measured slices, and every median was
approximately 4.194 million IDs/s. Differences among worker counts are inside
their observed ranges. Once the sequence space is exhausted, thread scheduling
around the next clock boundary can affect elapsed time but cannot increase
capacity.

The median build-plus-first-ID latency was 880 ns in both precision modes under
`RestartPolicy::Immediate`. These values are a setup baseline and do not
describe the explicit `WaitNextSlice` first-allocation delay. Tail values remain
sensitive to scheduling and interruption.

String conversion reduced observed synchronous dispatch throughput by roughly
26% to 31% relative to the corresponding numeric path, and asynchronous
throughput by roughly 11% to 16%. The concrete-versus-trait-object differences
were smaller and changed direction between sync and async cases, so this single
run is not evidence of a general dispatch advantage.

Reducing the sequence field by one bit would halve the hard limit to 2,048,000
IDs/s in millisecond mode and 2,097,152 IDs/s in second mode. That is a material
throughput reduction relative to this baseline.

## Limitations

- Results include the public API, mutex, system-clock reads, batching, and
  boundary-detection overhead; they are not isolated instruction-level
  measurements.
- Thread scheduling, CPU frequency, thermal state, and other system load can
  change repeated results.
- Real-clock slice alignment makes second-mode elapsed time sensitive to how
  quickly the final sequence capacity is consumed and the next slice begins.
- The benchmark measures one shared generator. Multiple independent hosts or
  sharded generator instances have different scaling behavior.
- Startup samples intentionally create independent instances with the same
  host. They measure latency only and do not assert uniqueness across instance
  replacement.

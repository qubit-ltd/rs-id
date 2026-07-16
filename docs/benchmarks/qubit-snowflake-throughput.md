# Qubit Snowflake Throughput Benchmark

## Scope

This document records a single-machine sustained-throughput and startup-latency
baseline for `QubitSnowflakeGenerator`. It is a reproducible observation on the
environment below, not a performance guarantee for other hardware, operating
systems, Rust versions, or workloads.

## Environment

- Date: 2026-07-16
- Branch: `codex/rs-id-generation-reliability`
- Repository base revision: `8f278f854833ab49cd9d613f5129b211e6409a95`
- Working tree: base revision plus the restart-policy, non-sleeping generation,
  raw rollback detection, `qubit-clock` injection, tests, benchmark
  organization, and documentation changes described in this report
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
from the generator instead of substituting a benchmark-only epoch. Each
throughput sample constructs a fresh generator, generates 100,000 untimed
warm-up IDs, aligns to a fresh clock slice, and then measures 2,000 millisecond
slices or 2 second slices. Every precision and worker-count combination is
sampled three times.

Workers generate IDs in batches of 64. Normal batches remain on the generation
hot path; only the final boundary batch is decoded so IDs outside the measured
timestamp range are not counted. Each sample asserts that its count does not
exceed the layout capacity: 4,096 IDs per millisecond slice or 4,194,304 IDs
per second slice. The tables report the sample with the median throughput, plus
the minimum and maximum throughput across all three samples.

Startup latency is measured separately over 10,000 fresh instances. Each
observation includes builder construction and the first `next_id` call. This
captures the immediate first allocation and is excluded from sustained
throughput timing.

## Command

```bash
RUSTFLAGS="-C target-cpu=native" \
    cargo bench --bench qubit_snowflake_throughput
```

The run reported this configuration:

```text
configuration throughput_samples=3 startup_samples=10000 warm_up_ids=100000
```

## Sustained Throughput Results

| Precision | Threads | Samples | Slices/sample | Capacity/sample | Median IDs | Median utilization | Median elapsed | Min IDs/s | Median IDs/s | Max IDs/s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Millisecond | 1 | 3 | 2,000 | 8,192,000 | 8,159,878 | 99.61% | 2.000126 s | 4,067,147 | 4,079,682 | 4,081,306 |
| Millisecond | 2 | 3 | 2,000 | 8,192,000 | 8,192,000 | 100.00% | 2.000169 s | 4,095,415 | 4,095,653 | 4,095,740 |
| Millisecond | 4 | 3 | 2,000 | 8,192,000 | 8,191,174 | 99.99% | 2.000144 s | 4,089,869 | 4,095,291 | 4,095,355 |
| Millisecond | 6 | 3 | 2,000 | 8,192,000 | 8,192,000 | 100.00% | 2.000183 s | 4,091,859 | 4,095,625 | 4,095,792 |
| Second | 1 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 1.999694 s | 4,194,205 | 4,194,945 | 4,195,563 |
| Second | 2 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 1.999821 s | 4,193,891 | 4,194,679 | 4,194,700 |
| Second | 4 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 1.999574 s | 4,194,172 | 4,195,197 | 4,195,524 |
| Second | 6 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.000073 s | 4,193,560 | 4,194,150 | 4,194,206 |

## Startup Latency Results

| Precision | Samples | Minimum | Median | Maximum |
|---|---:|---:|---:|---:|
| Millisecond | 10,000 | 117 ns | 131 ns | 25,997 ns |
| Second | 10,000 | 117 ns | 122 ns | 25,892 ns |

## Interpretation

Millisecond mode reached approximately 4.08 million IDs/s with one worker and
4.096 million IDs/s with two, four, and six workers. The two-worker sample had
the highest median, but the two-, four-, and six-worker min-to-max ranges
overlap and all three configurations are effectively at the 4,096 IDs/ms
layout limit. More callers cannot raise that hard limit and still contend for
the same generator mutex.

Second mode is also sequence-capacity-bound. Every sample consumed the complete
22-bit sequence space for both measured slices, and every median was
approximately 4.194 million IDs/s. Differences among worker counts are inside
their observed ranges. Once the sequence space is exhausted, thread scheduling
around the next clock boundary can affect elapsed time but cannot increase
capacity.

The median build-plus-first-ID latency was 131 ns in millisecond mode and 122 ns
in second mode, compared with 83 ns and 84 ns in the previous record. The
constructor now creates separate Arc-backed standard wall-clock, monotonic
clock, and blocking-sleeper objects instead of one clock closure, so a modest
startup-cost increase is expected. The min-to-max ranges overlap because tail
values are sensitive to scheduling and interruption; this run is not enough to
claim a general latency regression. Immediate first allocation still avoids a
time-slice startup fence.

For every throughput case, this run's complete min-to-max range was above the
corresponding previously recorded range on the same machine and unchanged
measurement method. That is an observed improvement for this working tree, not
a causal isolation of one implementation change or a guarantee for other
loads.

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

# Qubit Snowflake Throughput Benchmark

## Scope

This document records a single-machine sustained-throughput and startup-latency
baseline for `QubitSnowflakeGenerator`. It is a reproducible observation on the
environment below, not a performance guarantee for other hardware, operating
systems, Rust versions, or workloads.

## Environment

- Date: 2026-07-15
- Branch: `dev-starfish`
- Repository base revision: `e1fd56924fa4b6117a67646bddb68118e7fdc83f`
- Working tree: base revision plus the generator, benchmark, tests, and
  documentation described in this report
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

## Sustained Throughput Results

| Precision | Threads | Samples | Slices/sample | Capacity/sample | Median IDs | Median utilization | Median elapsed | Min IDs/s | Median IDs/s | Max IDs/s |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Millisecond | 1 | 3 | 2,000 | 8,192,000 | 6,458,930 | 78.84% | 2.000098 s | 3,062,183 | 3,229,306 | 3,273,197 |
| Millisecond | 2 | 3 | 2,000 | 8,192,000 | 6,322,251 | 77.18% | 2.000974 s | 3,002,746 | 3,159,587 | 3,351,284 |
| Millisecond | 4 | 3 | 2,000 | 8,192,000 | 5,932,524 | 72.42% | 2.001102 s | 2,920,390 | 2,964,628 | 3,232,600 |
| Millisecond | 6 | 3 | 2,000 | 8,192,000 | 5,977,238 | 72.96% | 2.000974 s | 2,635,216 | 2,987,165 | 3,306,038 |
| Second | 1 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.157073 s | 3,493,656 | 3,888,885 | 4,012,241 |
| Second | 2 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.005250 s | 4,121,096 | 4,183,322 | 4,192,669 |
| Second | 4 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.120366 s | 3,709,480 | 3,956,208 | 3,984,761 |
| Second | 6 | 3 | 2 | 8,388,608 | 8,388,608 | 100.00% | 2.447360 s | 3,424,117 | 3,427,615 | 3,461,218 |

## Startup Latency Results

| Precision | Samples | Minimum | Median | Maximum |
|---|---:|---:|---:|---:|
| Millisecond | 10,000 | 81 ns | 83 ns | 2,588 ns |
| Second | 10,000 | 81 ns | 84 ns | 349 ns |

## Interpretation

Millisecond mode reached its highest median result with one worker, at
approximately 3.23 million IDs/s. More callers did not improve median
throughput because they contend for the same generator mutex. The observed
min-to-max ranges also show why one benchmark sample is not sufficient for
comparing small differences between worker counts.

Second mode is sequence-capacity-bound. Every sample consumed the complete
22-bit sequence space for both measured slices. The best median result was
approximately 4.18 million IDs/s with two workers, close to the hard limit of
4,194,304 IDs per second. Once the sequence space is exhausted, thread
scheduling around the next clock boundary affects elapsed time but cannot
increase capacity.

The median build-plus-first-ID latency was below 100 ns for both precisions on
this run. This confirms that first generation no longer includes a one-second
startup fence. Tail values remain sensitive to scheduling and interruption, so
applications should not treat these observations as latency guarantees.

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

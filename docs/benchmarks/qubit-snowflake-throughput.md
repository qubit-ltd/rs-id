# Qubit Snowflake Throughput Benchmark

## Scope

This document records a single-machine sustained-throughput baseline for
`QubitSnowflakeGenerator`. It is a reproducible observation on the environment
below, not a performance guarantee for other hardware, operating systems, Rust
versions, or workloads.

## Environment

- Date: 2026-07-15
- Repository base revision: `612c97b61b99df98b7b69831cb698eeba9a1c56f`
- Working tree: base revision plus the benchmark and documentation described
  in this report
- CPU: Intel Core i5-9600K at 3.70 GHz
- Topology: 1 socket, 6 physical cores, 6 logical CPUs, no SMT
- Rust: 1.94.0 (`4a4ef493e 2026-03-02`)
- Cargo: 1.94.0 (`85eff7c80 2026-01-15`)
- Build profile: Cargo `bench` (optimized)
- Rust flags: `-C target-cpu=native`

## Method

The benchmark uses the real system clock and one sequential-mode Qubit
Snowflake generator shared by all worker threads. Generator construction and
the startup fence complete before timing. Each case starts at a fresh clock
slice and measures 5,000 millisecond slices or 5 second slices.

Workers generate IDs in batches of 64. Normal batches remain on the generation
hot path; only the final boundary batch is decoded so IDs outside the measured
timestamp range are not counted. Capacity follows the current layout: 4,096
IDs per millisecond slice and 4,194,304 IDs per second slice.

## Command

```bash
RUSTFLAGS="-C target-cpu=native" \
    cargo bench --bench qubit_snowflake_throughput
```

## Results

| Precision | Threads | IDs | Capacity | Utilization | Elapsed | Throughput |
|---|---:|---:|---:|---:|---:|---:|
| Millisecond | 1 | 16,952,951 | 20,480,000 | 82.78% | 5.000213 s | 3,390,446 IDs/s |
| Millisecond | 2 | 16,196,691 | 20,480,000 | 79.09% | 5.000090 s | 3,239,280 IDs/s |
| Millisecond | 4 | 16,706,170 | 20,480,000 | 81.57% | 5.000552 s | 3,340,865 IDs/s |
| Millisecond | 6 | 17,155,183 | 20,480,000 | 83.77% | 5.000864 s | 3,430,444 IDs/s |
| Second | 1 | 20,971,520 | 20,971,520 | 100.00% | 5.025570 s | 4,172,964 IDs/s |
| Second | 2 | 20,971,520 | 20,971,520 | 100.00% | 5.468831 s | 3,834,736 IDs/s |
| Second | 4 | 20,971,520 | 20,971,520 | 100.00% | 5.003777 s | 4,191,138 IDs/s |
| Second | 6 | 20,971,520 | 20,971,520 | 100.00% | 5.473521 s | 3,831,450 IDs/s |

## Interpretation

Millisecond mode reached its best result with six workers in this run, at
approximately 3.43 million IDs/s. The differences between worker counts are
small and not monotonic because every caller contends for the same generator
mutex; additional workers do not provide linear scaling.

Second mode is sequence-capacity-bound. Every case consumed the complete
22-bit sequence space for all five measured slices. One worker was already
enough to approach the 4,194,304 IDs/s hard limit, so additional workers cannot
increase sustained capacity and may increase boundary completion time.

Reducing the sequence field by one bit would halve the hard limit to 2,048,000
IDs/s in millisecond mode and 2,097,152 IDs/s in second mode. That is a material
throughput reduction relative to this baseline.

## Limitations

- Results include public API, mutex, clock, batching, and boundary-detection
  overhead; they are not isolated instruction-level latency measurements.
- Thread scheduling, CPU frequency, thermal state, and other system load can
  change repeated results.
- The benchmark measures one shared generator. Multiple independent hosts or
  sharded generator instances have different scaling behavior.
- Startup latency is intentionally excluded from throughput and must be
  measured separately when it matters to an application.

# Qubit ID User Guide

[中文用户手册](user_guide.zh_CN.md) · [README](../README.md) · [API documentation](https://docs.rs/qubit-id)

## Purpose and audience

This guide is for service developers choosing an ID generator for a Rust
application. It focuses on the three Snowflake-family generators in `qubit-id`:
`ClassicalSnowflakeGenerator`, `SnowflakeGenerator`, and `SonyflakeGenerator`.
It explains how the layouts affect capacity, ordering, storage compatibility,
and deployment identity. The examples target `qubit-id` 0.3 and Rust 1.94 or
later.

## Conceptual model

Every Snowflake-family ID combines three kinds of information:

```text
time slice + deployment identity + sequence within that time slice
```

The generator owns allocation state. A shared live generator serializes
allocation, increments the sequence inside a time slice, and waits when a slice
or retryable clock condition cannot make immediate progress. The layout decides
which bits represent time, identity, and sequence; the builder decides the
epoch, clock tolerance, restart policy, and injected clock capabilities.

The theoretical throughput below is the capacity of the sequence field. It is
not a benchmark guarantee. A real application also pays for synchronization,
clock sampling, contention, and retry waits.

## Practical scenario: choosing an order ID layout

An order service is deployed on multiple hosts and needs numeric IDs that can
be stored and transported consistently. Its decision criteria are:

1. How many independent hosts or machines must allocate IDs?
2. How many IDs can one identity allocate in one time slice?
3. How long must one epoch remain valid?
4. Must the ID stay below `i64::MAX` for signed database keys?
5. Must the layout be configurable or hide direct numeric time ordering?

Use the comparison first, then configure an exclusive host, node, or machine ID
for every concurrently active generator in the same namespace.

## Layout comparison

| Generator and mode | Bit layout | Time unit | Identity count | Per-identity theoretical throughput | Ideal aggregate capacity | Time range |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Classical Snowflake | `[reserved 1][time 41][node 10][sequence 12]` | 1 ms | 1,024 nodes | 4,096/ms, about 4.096 million/s | About 4.194 billion/s | About 69.7 years |
| Qubit, millisecond | `[mode 1][precision 1][time 41][host 9][sequence 12]` | 1 ms | 512 hosts | 4,096/ms, about 4.096 million/s | About 2.097 billion/s | About 69.7 years |
| Qubit, second | `[mode 1][precision 1][time 31][host 9][sequence 22]` | 1 s | 512 hosts | 4,194,304/s, about 4.194 million/s | About 2.147 billion/s | About 68.1 years |
| Sonyflake default | `[reserved 1][time 39][sequence 8][machine 16]` | 10 ms | 65,536 machines | 256/10 ms, about 25,600/s | About 1.678 billion/s | About 174.8 years |

The ideal aggregate column assumes every identity is active and independent.
It is useful for capacity planning, not for sizing a single shared generator.
For Sonyflake, the configurable formulas are:

```text
time_bits     = 63 - sequence_bits - machine_bits
identity_count = 2^machine_bits
throughput     = 2^sequence_bits / time_unit
time_range     = 2^time_bits × time_unit
```

### Classical Snowflake

The layout is fixed at 41 millisecond timestamp bits, 10 node bits, and 12
sequence bits. It is the best default when interoperability with a traditional
Snowflake-style 63-bit layout matters more than custom capacity planning.

Within one millisecond, numeric IDs are arranged by timestamp, then node, then
sequence. This preserves the familiar time-oriented shape, but IDs from
different nodes are not a strict global issuance order.

### Qubit Snowflake

Qubit adds two self-describing header bits:

- `mode`: `Sequential` stores timestamp bits normally; `Spread` reverses the
  timestamp bits within the selected width.
- `precision`: `Millisecond` uses 41 timestamp bits and 12 sequence bits;
  `Second` uses 31 timestamp bits and 22 sequence bits.

The selected mode and precision are encoded in every ID, so decoding does not
need a separately configured layout. `Spread` reduces the direct numeric
relationship between adjacent time slices, but it is reversible obfuscation,
not encryption. It does not hide the sequence behavior inside one time slice.

Qubit `Spread` always sets bit 63. Such IDs exceed `i64::MAX` and must not be
cast to a signed database key. Use `Id`, unsigned decimal text, or 8-byte
binary transport instead.

### Sonyflake

Sonyflake defaults to 39 elapsed-time bits in 10-millisecond units, 8 sequence
bits, and 16 machine bits. Its default favors machine count and lifetime over
per-machine burst capacity. The builder can change sequence and machine widths;
the remaining bits become the time field, subject to the layout's validation
rules.

Sonyflake's low fields are ordered as sequence then machine. Therefore, for the
same elapsed-time value, numeric order groups by sequence before machine. All
services decoding the ID must agree on the complete bit-width and time-unit
configuration.

## Installation and minimal configuration

Enable the default Qubit generator:

```toml
[dependencies]
qubit-id = "0.3"
```

Select a different Snowflake implementation with features:

```toml
qubit-id = { version = "0.3", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.3", default-features = false, features = ["sonyflake"] }
```

The required constructor identity is called `host`, `node_id`, or `machine_id`
according to the selected implementation:

```rust
use qubit_id::{ClassicalSnowflakeGenerator, IdGenerationError};

fn main() -> Result<(), IdGenerationError> {
    let generator = ClassicalSnowflakeGenerator::new(7)?;
    let id = generator.generate()?;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

Use the builder when the epoch, clock tolerance, restart policy, or injected
clock must be explicit. `ClassicalSnowflakeGenerator` and Qubit Snowflake use
the documented Qubit epoch by default; Sonyflake has its own default epoch.
When interoperating with an existing namespace, configure the same epoch and
layout on every decoder and generator.

## Core workflow

1. Select one feature and one layout.
2. Allocate a unique identity within that layout's namespace.
3. Construct one shared generator per identity and process boundary.
4. Use `generate()` when the generator should wait, `try_generate()` when the
   caller owns retry scheduling, or `generate_async()` in an asynchronous flow.
5. Store and transport the result according to the selected layout's signedness
   and serialization rules.

All three Snowflake generators expose the same primary methods:
`new(...)`, `builder(...)`, `layout()`, `epoch()`, `expires_at()`,
`max_clock_skew()`, `try_generate()`, `generate()`, `generate_async()`, and
`compose_at(time, sequence)`.

The concrete asynchronous method has an unboxed outer Future. The object-safe
`AsyncIdGenerator` trait uses a boxed Future so it can cross a dynamic injection
boundary. Neither path requires Tokio by itself; the injected `Timer` supplies
the runtime-specific waiting behavior.

The generator traits use generic `Output` and `Error` parameters. They default
to `Id` and `IdGenerationError`, so the common synchronous injection boundary
can be written as:

```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn create_generator() -> Result<Arc<dyn IdGenerator<Id>>, IdGenerationError> {
    Ok(Arc::new(SnowflakeGenerator::new(7)?))
}
```

Custom generator types can specify both parameters, for example
`IdGenerator<String, MyError>`.

## Advanced usage

### Changing Sonyflake capacity

If a deployment needs more IDs per time slice, increase `bits_sequence`. If it
needs more machines, increase `bits_machine`. Both choices consume time bits or
each other, so publish the complete layout configuration with the service
contract. Do not change these settings after IDs have entered a shared namespace
unless all consumers are versioned for the new layout.

### Choosing Qubit precision and mode

Use `TimestampPrecision::Millisecond` when the ID must carry millisecond-level
elapsed time. Use `TimestampPrecision::Second` when one-second time slices are
acceptable and a larger sequence range is more valuable. Use
`IdMode::Spread` only when the application benefits from weaker direct numeric
time correlation and can store unsigned 64-bit values.

### Deterministic clocks in tests

Derive the wall clock and timer from one `ManualMonotonicClock` so tests can
advance time after observing a registered retry deadline:

```text
let clock = ManualMonotonicClock::new_shared();
let generator = SnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build()?;
```

This avoids real sleeps and makes sequence rollover, clock rollback, and
expiration tests repeatable.

## Errors and diagnostics

Builders validate the layout and lifetime before returning a generator. The
most relevant errors are:

- `EpochAhead`: the configured epoch is later than the injected wall clock.
- `GeneratorExpired`: `now >= expires_at`, or a live generator reaches its
  exclusive expiration boundary.
- `ClockMovedBackwards`: raw wall-clock rollback exceeds `max_clock_skew(...)`.
- `WaitFailed`: the timer cannot register or complete a retry wait.
- Layout-specific range errors such as `NodeOutOfRange`, `HostOutOfRange`,
  `MachineIdOutOfRange`, `TimestampOverflow`, or `SequenceOverflow`.
- `RandomSourceFailed` for UUID v4 generation when the operating system cannot
  provide random bytes.

These conditions return `IdGenerationError`; they are not panic-based control
flow. Log the selected layout, epoch, identity, and current time slice when
diagnosing a namespace collision or expiration problem, while avoiding any
assumption that an ID is a security token.

## Troubleshooting

### The generator returns `GeneratorExpired`

Check the configured epoch, the selected precision or Sonyflake time unit, and
the value returned by `expires_at()`. A shorter time unit or an older epoch can
make the representable lifetime end sooner.

### IDs collide after a restart

Verify that concurrently active processes do not reuse the same host, node, or
machine identity. Confirm that epoch and layout configuration match the intended
namespace. `RestartPolicy::WaitNextSlice` can reduce same-slice reuse after a
replacement, but it does not persist allocation state or coordinate concurrent
same-identity processes.

### IDs cannot be stored in a signed database column

Use Classical Snowflake, Sonyflake, or Sequential Qubit when the selected value
remains below `i64::MAX`. Qubit `Spread` sets bit 63 and requires an unsigned
representation, decimal text, or binary storage.

### Generation waits indefinitely

`generate()` may wait while the wall clock stalls, a sequence range is exhausted,
or a retryable rollback requires time to advance. Check that the injected timer
backend is driven independently of the caller thread. Use `try_generate()` when
the application must own scheduling and backpressure.

## Limitations and best practices

- The generator does not persist allocation state and does not provide a
  distributed identity lease.
- Every concurrently active generator in one namespace needs an exclusive
  identity. Sharing one identity across hosts defeats the layout's uniqueness
  assumption.
- Timestamp order is not a security boundary. Classical, Sequential Qubit, and
  Sonyflake IDs expose time structure; Qubit `Spread` only applies reversible
  obfuscation.
- The theoretical capacities are field limits. Benchmark the complete service
  path before making an operational throughput promise.
- Keep the exact epoch and layout configuration in the service contract so
  producers, consumers, and migration tools decode the same namespace.
- `Id` is a transparent `u64` value. With `serde`, human-readable serialization
  uses decimal strings and compact serialization uses `u64`.

## Further reading

- [README](../README.md)
- [中文 README](../README.zh_CN.md)
- [中文用户手册](user_guide.zh_CN.md)
- [API documentation](https://docs.rs/qubit-id)

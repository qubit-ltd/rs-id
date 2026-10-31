# Qubit ID User Guide

[中文用户手册](user_guide.zh_CN.md) · [README](../README.md) · [API documentation](https://docs.rs/qubit-id)

## Purpose and audience

This guide is for service developers choosing an ID generator for a Rust
application. It focuses on the three Snowflake-family generators in `qubit-id`:
`ClassicalSnowflakeGenerator`, `SnowflakeGenerator`, and `SonyflakeGenerator`.
It explains how the layouts affect capacity, ordering, storage compatibility,
and deployment identity. The examples target the `qubit-id` 0.7 series (with
0.7.2 as the version checked for this guide) and Rust 1.94 or later.

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
for every concurrently active generator in the same namespace. The application
or deployment system must coordinate these assignments across processes; the
generator cannot prevent separate processes from reusing a node identifier.

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
qubit-id = "0.7"
```

For an application that uses `TokioTimer`, declare the timer and runtime in the
application's own dependencies. These are not production features of `qubit-id`:

```toml
[dependencies]
qubit-id = "0.7"
qubit-clock = { version = "0.13", features = ["tokio"] }
tokio = { version = "1.54", features = ["macros", "rt", "time"] }
```

Select a different Snowflake implementation with features:

```toml
qubit-id = { version = "0.7", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.7", default-features = false, features = ["sonyflake"] }
```

The required constructor identity is called `host`, `node_id`, or `machine_id`
according to the selected implementation:

<!-- qubit-id-example features="classic-snowflake" mode="run" -->
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
Epoch is not encoded as a namespace tag. At wall time E + 10.5 seconds,
second-precision generators with the same host and layout but epochs E and
E + 1 ms both encode timestamp 10 and sequence 0, so they can return the same
ID. Changing epoch is not an isolation boundary: assess overlap with historical
values, and use an external partition or storage namespace when isolation is
required. When interoperating with an existing namespace, configure compatible
epoch and layout semantics on every decoder and generator.

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use qubit_id::IdGenerationError;
use qubit_id::SnowflakeGenerator;

fn main() {
    assert!(matches!(
        SnowflakeGenerator::new(512),
        Err(IdGenerationError::HostOutOfRange { host: 512, max: 511 })
    ));
}
```

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

The default rollback tolerance differs by generator:

| Generator | Default `max_clock_skew` |
| --- | ---: |
| Qubit Snowflake | 3 seconds |
| Classical Snowflake | zero |
| Sonyflake | zero |

The tolerance is measured against raw wall-clock rollback, even when both
observations fall in the same encoded time slice. With zero tolerance, any
observed rollback is an error. A rollback within the configured tolerance makes
`try_generate()` return a retry result; `generate()` waits synchronously and
`generate_async()` waits asynchronously. A rollback beyond the tolerance
returns `ClockMovedBackwards`. Set the tolerance explicitly when this policy
needs to be part of the deployment configuration:

<!-- qubit-id-example features="classic-snowflake" mode="run" -->
```rust
use std::time::Duration;
use qubit_id::{ClassicalSnowflakeGenerator, IdGenerationError};

fn main() -> Result<(), IdGenerationError> {
    let generator = ClassicalSnowflakeGenerator::builder(7)
        .max_clock_skew(Duration::from_millis(5))
        .build()?;
    assert_eq!(generator.max_clock_skew(), Duration::from_millis(5));
    Ok(())
}
```

This example checks the configured value; it does not simulate a rollback.
Existing generator tests cover rollback behavior.

The concrete asynchronous method has an unboxed outer Future. The object-safe
`AsyncIdGenerator` trait uses a boxed Future so it can cross a dynamic injection
boundary. Neither path requires Tokio by itself; the injected `Timer` supplies
the runtime-specific waiting behavior. `TryIdGenerator` does not wait for time
to advance, but allocation may briefly use a synchronous lock. `AsyncIdGenerator`
awaits timer progress when needed, while its allocation attempt may also use a
synchronous lock. Neither promises lock-free or wait-free calls. A custom
`WallClock` is sampled inside the allocation critical section; `now()` should
return quickly, avoid blocking I/O, and not re-enter the same generator.

The generator traits use generic `Output` and `Error` parameters. They default
to `Id` and `IdGenerationError`, so the common synchronous injection boundary
can be written as:

<!-- qubit-id-example features="qubit-snowflake" mode="check" -->
```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn create_generator() -> Result<Arc<dyn IdGenerator<Id>>, IdGenerationError> {
    Ok(Arc::new(SnowflakeGenerator::new(7)?))
}
```

Custom generator types can specify both parameters, for example
`IdGenerator<String, MyError>`.

### Asynchronous generation with Tokio

`qubit-id` is runtime-neutral. This example creates `TokioTimer` inside the
Tokio runtime, and `WaitNextSlice` means generation may actually wait for the
next time slice. The five-second timeout is an application-level budget.
`generate()` is a blocking API; use `generate_async()` for asynchronous work.

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use std::sync::Arc;
use std::time::Duration;
use qubit_clock::TokioTimer;
use qubit_id::{AsyncIdGenerator, Id, RestartPolicy, SnowflakeGenerator, TimestampPrecision};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Arc::new(TokioTimer::try_current()?);
    let generator: Arc<dyn AsyncIdGenerator<Id>> = Arc::new(
        SnowflakeGenerator::builder(7)
            .precision(TimestampPrecision::Millisecond)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .timer(timer)
            .build()?,
    );
    let id = tokio::time::timeout(Duration::from_secs(5), generator.generate_async()).await??;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

### Scheduling retries with `try_generate`

Use `Timer` to wait only when the generator returns `RetryAfter`. Other
generation, timer, and timeout errors propagate to the caller. The timeout is
the application's total wait budget.

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use std::sync::Arc;
use std::time::Duration;
use qubit_clock::{Timer, TokioTimer};
use qubit_id::{GenerationAttempt, Id, RestartPolicy, SnowflakeGenerator, TimestampPrecision};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timer = Arc::new(TokioTimer::try_current()?);
    let generator = SnowflakeGenerator::builder(7)
        .precision(TimestampPrecision::Millisecond)
        .restart_policy(RestartPolicy::WaitNextSlice)
        .timer(timer.clone())
        .build()?;
    let id = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match generator.try_generate()? {
                GenerationAttempt::Generated(id) => {
                    return Ok::<Id, Box<dyn std::error::Error>>(id);
                }
                GenerationAttempt::RetryAfter { delay } => {
                    timer.after(delay)?.await?;
                }
            }
        }
    }).await??;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

## Advanced usage

### Changing Sonyflake capacity

If a deployment needs more IDs per time slice, increase `bits_sequence`. If it
needs more machines, increase `bits_machine`. Both choices consume time bits or
each other, so publish the complete layout configuration with the service
contract. Do not change these settings after IDs have entered a shared namespace
without a migration plan. Versioning consumers preserves decoding compatibility;
it does not make old and new numeric IDs disjoint. With the same epoch and time
unit, machine ID 7 and sequence 0 produce `16777223` both for the old layout
`(bits_sequence=8, bits_machine=16)` at elapsed time 1 and for the new layout
`(8, 15)` at elapsed time 2. Replacing the old generator before starting the
new one can therefore reuse a historical numeric ID even when every consumer
decodes each layout correctly. A changed epoch is not automatic isolation, and
recording a version only in decoder configuration does not protect a database
whose unique key is the bare ID. Keep the existing configuration in a shared
bare-ID namespace, or migrate to an independent namespace, include an external
version in the storage key, or prove that the old and new allocation ranges do
not overlap.

For `SonyflakeLayout::new` and the builder, a zero `bits_sequence` or
`bits_machine` selects its default width (8 and 16 bits respectively). Each
explicit nonzero width must be less than 31, and the normalized sequence and
machine widths must leave at least 32 time bits (`bits_time = 63 - bits_sequence
- bits_machine`). The time unit must be at least 1 ms, and the machine ID must
fit its field. A layout can pass these checks while its lifetime is still
unrepresentable or already expired for the selected epoch and current time;
the builder validates both before returning a generator. See
[`IdGenerationError`](../src/id_generation_error.rs) for structured errors.

This runnable example configures the new Sonyflake namespace and checks its
layout, encoded fields, and expiration boundary:

<!-- qubit-id-example features="sonyflake" mode="run" -->
```rust
use std::time::Duration;
use qubit_id::{IdGenerationError, SonyflakeGenerator, SonyflakeLayout};

fn main() -> Result<(), IdGenerationError> {
    let generator = SonyflakeGenerator::builder(7)
        .bits_sequence(12)
        .bits_machine(12)
        .time_unit(Duration::from_millis(10))
        .build()?;
    let layout = generator.layout();
    assert_eq!(layout.bits_time(), 39);
    assert_eq!(layout.bits_sequence(), 12);
    assert_eq!(layout.bits_machine(), 12);
    assert_eq!(layout.time_unit(), Duration::from_millis(10));
    assert_eq!(layout.max_sequence(), 4095);
    assert_eq!(layout.max_machine_id(), 4095);
    let id = layout.compose(42, 3)?;
    let parts = layout.decode(id);
    assert_eq!(parts.elapsed_time(), 42);
    assert_eq!(parts.sequence(), 3);
    assert_eq!(parts.machine_id(), 7);
    assert_eq!(generator.expires_at(), layout.expires_at(generator.epoch())?);
    assert!(generator.expires_at() > generator.epoch());
    assert!(matches!(
        SonyflakeLayout::new(7, 16, 16, Duration::from_millis(10)),
        Err(IdGenerationError::InvalidBitLength { name: "time", .. })
    ));
    Ok(())
}
```

Twelve sequence bits and twelve machine bits each represent 4,096 values. The
39 time bits with a 10 ms unit span about 174.8 years; these capacities do not
promise generation throughput. `layout.compose(...)` combines supplied fields
for this check and does not allocate or reserve a unique ID. This configuration
belongs to a new namespace and must not be substituted directly for a shared
old namespace; follow the migration choices above to avoid reusing numeric IDs.

### Choosing Qubit precision and mode

Use `TimestampPrecision::Millisecond` when the ID must carry millisecond-level
elapsed time. Use `TimestampPrecision::Second` when one-second time slices are
acceptable and a larger sequence range is more valuable. Use
`IdMode::Spread` only when the application benefits from weaker direct numeric
time correlation and can store unsigned 64-bit values.

### Deterministic clocks in tests

The following standalone program derives the wall clock and timer from one
`ManualMonotonicClock`. Its observer confirms that the retry deadline is
registered before `advance_to_next_deadline()` advances logical time. On
success, its assertions pass and the program exits. The timeout only protects
against a hung example; it does not advance logical time. `WaitNextSlice`
deliberately causes one wait here and does not provide persistence or
cross-process uniqueness.

Use these exact dependencies. The clock crate's `test-util` feature is not
needed; Tokio's `time` feature is enabled on Tokio itself:

```toml
[dependencies]
qubit-id = "0.7"
qubit-clock = "0.13"
tokio = { version = "1.54", features = ["macros", "rt", "time"] }
```

<!-- qubit-id-example features="qubit-snowflake" mode="run" -->
```rust
use std::sync::Arc;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::MonotonicClock;
use qubit_id::RestartPolicy;
use qubit_id::SnowflakeGenerator;
use qubit_id::TimestampPrecision;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let epoch = UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    let initial_time = epoch + Duration::from_secs(10);
    let clock = ManualMonotonicClock::new_shared();
    let generator = Arc::new(
        SnowflakeGenerator::builder(7)
            .epoch(epoch)
            .precision(TimestampPrecision::Millisecond)
            .restart_policy(RestartPolicy::WaitNextSlice)
            .wall_clock(clock.new_wall_clock(initial_time))
            .timer(clock.new_timer())
            .build()?,
    );
    let worker_generator = Arc::clone(&generator);
    let task = tokio::spawn(async move { worker_generator.generate_async().await });
    let observed = tokio::time::timeout(
        Duration::from_secs(5),
        clock.wait_for_next_deadline_async(),
    )
    .await?;
    assert_eq!(observed.elapsed_since_origin(), Duration::from_millis(1));
    assert!(!task.is_finished());
    let reached = clock.advance_to_next_deadline().expect("registered deadline");
    assert_eq!(reached, observed);
    let id = tokio::time::timeout(Duration::from_secs(5), task).await???;
    let expected = generator.compose_at(initial_time + Duration::from_millis(1), 0)?;
    assert_eq!(id, expected);
    Ok(())
}
```

The one-millisecond assertion verifies the wait duration, and comparing the
returned ID with the ID composed at the new time slice verifies sequence zero.

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

Blocking and asynchronous APIs wait automatically for retryable slice progress.
If the clock moves backwards beyond the configured tolerance, keep the original
generator and let its retained high-water mark recover as the clock catches up;
replacing it with another generator using the same identity does not safely
clear that state. A `WaitFailed` error requires recovery appropriate to the
injected Timer backend. Cancellation is an application control-flow decision
and should be handled separately from generation errors. Canceling an
incomplete asynchronous wait preserves the generator's shared allocation
state; cancellation also cannot revoke IDs already returned to callers.
`EpochAhead` and
`GeneratorExpired` require correcting the configuration or deploying a valid
new namespace; they are not transient retry signals.

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
same-identity processes. This policy reduces sequential-replacement risk only
when the replacement's first observed slice is not earlier than the
predecessor's last allocated slice. It does not bound the elapsed time of the
first allocation; clock rollback and scheduling can extend the wait.

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
- `Id::to_padded_decimal()` returns a fixed-width 20-digit decimal string for
  persistent text keys whose lexicographic order must match numeric `u64` order.
  Ordinary `Display` remains unpadded.
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

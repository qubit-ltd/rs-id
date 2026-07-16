# Qubit ID (`rs-id`)

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Docs.rs](https://docs.rs/qubit-id/badge.svg)](https://docs.rs/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Documentation: [API Reference](https://docs.rs/qubit-id)

`qubit-id` provides ID generation utilities for Rust services.

It includes one common `IdGenerator` trait with associated ID and error types,
plus generators for
database-friendly Snowflake IDs, Sonyflake-style IDs, and fast UUID-like random
identifiers.

## Why Use It

Use `qubit-id` when you need:

- Qubit Snowflake IDs with fixed high-bit mode and precision headers
- classic Snowflake IDs with a compact 64-bit numeric representation
- Sonyflake-style IDs with longer runtime under small sequence pressure
- fast UUID-like random strings
- one trait-based API that can return typed IDs and string representations

## Installation

The default feature set enables only `qubit-snowflake`. The common
`IdGenerator`, `GenerationOutcome`, and `IdError` APIs are always available;
each non-default algorithm is opt-in.

| Feature | Enabled by default | API |
| --- | --- | --- |
| `qubit-snowflake` | yes | Qubit Snowflake layout, parts, builder, and generator |
| `classic-snowflake` | no | Classic Snowflake layout, parts, builder, and generator |
| `sonyflake` | no | Sonyflake layout, parts, builder, and generator |
| `uuid` | no | Mica UUID-like generator and string helpers |

Use the default Qubit Snowflake API:

```toml
[dependencies]
qubit-id = "0.3"
```

Or select one optional algorithm without the default:

```toml
[dependencies]
qubit-id = { version = "0.3", default-features = false, features = ["classic-snowflake"] }
```

```toml
[dependencies]
qubit-id = { version = "0.3", default-features = false, features = ["sonyflake"] }
```

```toml
[dependencies]
qubit-id = { version = "0.3", default-features = false, features = ["uuid"] }
```

Features can be combined in one dependency declaration. Use
`default-features = false` with no feature list when only the common core API
is needed.

## Quick Start

```rust
use qubit_id::{
    GenerationOutcome, IdGenerator, QubitSnowflakeGenerator,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snowflake = QubitSnowflakeGenerator::new(1)?;
    let id: u64 = snowflake.next_id()?;
    let id_text = snowflake.next_string()?;

    match snowflake.try_next_id()? {
        GenerationOutcome::Generated(id) => println!("{id}"),
        GenerationOutcome::RetryAfter(duration) => {
            println!("retry after {duration:?}");
        }
    }

    println!("{id} {id_text}");
    Ok(())
}
```

## Core API At A Glance

| Type | Purpose |
| --- | --- |
| `IdGenerator` | Common trait with associated ID and error types for generation and string formatting. |
| `GenerationOutcome<T>` | Result of one non-sleeping generation attempt: an ID or a retry delay. |
| `RestartPolicy` | Controls whether a fresh Snowflake generator allocates immediately or waits for the next time slice. |
| `QubitSnowflakeGenerator` | Qubit fixed-header Snowflake generator. |
| `QubitSnowflakeGeneratorBuilder` | Configures a Qubit Snowflake generator. |
| `QubitSnowflakeLayout` | Composes Qubit Snowflake IDs and decodes any Qubit layout from its fixed header. |
| `QubitSnowflakeParts` | Fields returned by `QubitSnowflakeLayout::decode`. |
| `SnowflakeGenerator` | Classic 41-bit time, 10-bit node, 12-bit sequence Snowflake generator. |
| `SnowflakeGeneratorBuilder` | Configures a classic Snowflake generator. |
| `SnowflakeLayout` | Composes and decodes classic Snowflake IDs. |
| `SnowflakeParts` | Fields returned by `SnowflakeLayout::decode`. |
| `SonyflakeGenerator` | Sonyflake-style generator with configurable sequence and machine bits. |
| `SonyflakeGeneratorBuilder` | Configures a Sonyflake-style generator. |
| `SonyflakeLayout` | Composes and decodes Sonyflake IDs using a configured layout. |
| `SonyflakeParts` | Fields returned by `SonyflakeLayout::decode`. |
| `MicaUuidLikeGenerator` | Mica-style random 128-bit UUID-like generator. |
| `fast_uuid_like` | Generates canonical lowercase UUID-like text. |
| `fast_simple_uuid_like` | Generates compact lowercase 32-hex UUID-like text. |

## Uniqueness And Deployment

The three Snowflake-family generators are thread-safe. Successful `next_id`
and `next_string` calls on one shared live generator instance never repeat an
ID. Within a process, share one generator instance for each ID namespace rather
than constructing one per thread or request.

Across processes and servers, every concurrently running generator instance
that can emit into the same namespace must have an exclusive identity:

- `host` for `QubitSnowflakeGenerator`
- `node_id` for `SnowflakeGenerator`
- `machine_id` for `SonyflakeGenerator`

The crate does not allocate or coordinate these identities. Different epochs,
start times, or bit layouts can also overlap numerically, so deployment
configuration is part of the ID namespace.

The default restart policy is `RestartPolicy::Immediate`: the first call
allocates sequence zero in the currently observed logical time slice without
waiting. Allocation state is not persisted. State loss or replacement can
repeat an ID only when all three conditions hold: the instances use the same
effective identity, layout, and reference time; they allocate in the same
logical time slice; and their allocated sequence ranges overlap. The effective
identity is `host`, `node_id`, or `machine_id`; the reference time is the epoch
or Sonyflake start time.

`RestartPolicy::WaitNextSlice` records the first slice observed by a fresh
generator and waits for a later slice before allocating. It reduces duplicate
risk only for a sequential replacement where the replacement's first observed
slice is not earlier than the predecessor's last allocated slice. The crate does
not persist that predecessor watermark, so clock rollback across a restart can
make the replacement re-enter a slice already used by its predecessor and
repeat IDs. It also does not coordinate concurrently running instances with the
same effective identity: such instances can cross the fence together and
allocate overlapping sequence ranges. Concurrent same-identity deployment still
requires an external exclusive lease or equivalent coordination. This crate
does not provide persistent allocation state or cross-process coordination.

`try_next_id()` and `try_next_string()` perform one allocation attempt and
never sleep for clock progress or invoke the configured sleeper. A synchronized
generator can still wait briefly to acquire its internal mutex. `next_id()` and
`next_string()` adapt retry outcomes into blocking waits. After sequence
exhaustion or a tolerated
Qubit clock rollback, they normally wait for approximately one configured time
unit, but they may wait indefinitely when the wall clock stalls or an injected
sleeper does not cause the wall clock to progress. Qubit Snowflake retries
rollback within its configured tolerance and rejects larger skew; classic
Snowflake and Sonyflake reject any observed rollback immediately.

`compose`, `generate_at`, and `decode` are stateless transformations and do not
guarantee uniqueness. Decoding an arbitrary `u64` only extracts fields; it does
not authenticate or validate the value. `MicaUuidLikeGenerator` uses 128 random
bits, so its uniqueness is probabilistic and a theoretical collision remains
possible.

## Snowflake Lifetime

`expires_at()` returns the exclusive expiration boundary. Every Snowflake
layout accepts timestamps from zero through its maximum timestamp, so the
boundary is one complete time unit after the last representable timestamp.
The generator caches that boundary and exposes it without recalculation.

```rust
use qubit_id::QubitSnowflakeGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = QubitSnowflakeGenerator::new(1)?;
    let calculated = generator.layout().expires_at(generator.epoch())?;

    assert_eq!(generator.expires_at(), calculated);
    Ok(())
}
```

Construction samples the configured wall clock. When
`now >= expires_at`, construction panics because the application cannot emit
a valid timestamp with that configuration. This applies to `new()` and
builder `build()` paths. If the exclusive boundary itself cannot be
represented by `SystemTime`, layout calculation and builder construction
return `IdError::ExpirationTimeOverflow` instead. For Qubit and classic
Snowflake the origin is `epoch`; for Sonyflake it is `start_time`.

## Generator Examples

### QubitSnowflakeGenerator

Use `QubitSnowflakeGenerator` for the Qubit fixed-header Snowflake layout. The
default constructor uses sequential mode, second precision, and the default
Qubit epoch.

```rust
use qubit_id::{
    IdGenerator, QubitSnowflakeGenerator, QubitSnowflakeLayout,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The argument is the 9-bit host ID encoded into generated IDs.
    // It must be in the range 0..=511.
    let generator = QubitSnowflakeGenerator::new(42)?;

    let id = generator.next_id()?;
    let id_text = generator.next_string()?;

    let parts = QubitSnowflakeLayout::decode(id);
    assert_eq!(parts.host(), 42);

    println!("{id} {id_text}");
    Ok(())
}
```

Configure the Qubit layout explicitly when you need spread mode or millisecond
precision. The example also selects `WaitNextSlice`; omit that setter to keep
the default `Immediate` policy.

```rust
use std::time::{Duration, UNIX_EPOCH};

use qubit_id::{
    IdGenerator, IdMode, QubitSnowflakeGenerator, QubitSnowflakeLayout,
    RestartPolicy, TimestampPrecision,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = QubitSnowflakeGenerator::builder(7)
        .mode(IdMode::Spread)
        .precision(TimestampPrecision::Millisecond)
        .epoch(UNIX_EPOCH + Duration::from_millis(1_543_708_800_000))
        .restart_policy(RestartPolicy::WaitNextSlice)
        .build()?;

    let id = generator.next_id()?;
    let parts = QubitSnowflakeLayout::decode(id);

    assert_eq!(parts.mode(), IdMode::Spread);
    assert_eq!(parts.precision(), TimestampPrecision::Millisecond);
    assert_eq!(parts.host(), 7);

    Ok(())
}
```

### SnowflakeGenerator

Use `SnowflakeGenerator` when you need the classic 41-bit millisecond timestamp,
10-bit node, and 12-bit sequence layout. This API requires the
`classic-snowflake` feature.

```rust
use qubit_id::{IdGenerator, SnowflakeGenerator, SnowflakeLayout};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SnowflakeGenerator::new(3)?;

    let id = generator.next_id()?;

    assert_eq!(SnowflakeLayout::decode(id).node_id(), 3);
    println!("{id}");

    Ok(())
}
```

You can also compose and inspect deterministic IDs from known parts.

```rust
use qubit_id::SnowflakeLayout;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let layout = SnowflakeLayout::new(3)?;
    let id = layout.compose(1_000, 5)?;
    let parts = SnowflakeLayout::decode(id);

    assert_eq!(parts.timestamp(), 1_000);
    assert_eq!(parts.node_id(), 3);
    assert_eq!(parts.sequence(), 5);

    Ok(())
}
```

### SonyflakeGenerator

Use `SonyflakeGenerator` when a larger machine ID space matters more than
per-machine burst throughput. This API requires the `sonyflake` feature.

```rust
use qubit_id::{IdGenerator, SonyflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SonyflakeGenerator::new(65_535)?;

    let id = generator.next_id()?;

    assert_eq!(generator.layout().decode(id).machine_id(), 65_535);
    println!("{id}");

    Ok(())
}
```

For custom deployments, configure the sequence bits, machine bits, time unit,
and start time explicitly.

```rust
use std::time::{Duration, UNIX_EPOCH};

use qubit_id::{IdGenerator, SonyflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SonyflakeGenerator::builder(15)
        .bits_sequence(10)
        .bits_machine(14)
        .time_unit(Duration::from_millis(1))
        .start_time(UNIX_EPOCH + Duration::from_secs(1_735_689_600))
        .build()?;

    let id = generator.next_id()?;

    assert_eq!(generator.layout().bits_sequence(), 10);
    assert_eq!(generator.layout().bits_machine(), 14);
    assert_eq!(generator.layout().decode(id).machine_id(), 15);

    Ok(())
}
```

### MicaUuidLikeGenerator And Helpers

Use `MicaUuidLikeGenerator` when you want a random 128-bit value with UUID-like
lowercase text formatting. Use the helper functions when you only need strings.
These APIs require the `uuid` feature.

```rust
use qubit_id::{
    IdGenerator, MicaUuidLikeGenerator, fast_simple_uuid_like, fast_uuid_like,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = MicaUuidLikeGenerator::new();

    let value = generator.next_id()?;
    let canonical = generator.format_id(&value);
    let compact = MicaUuidLikeGenerator::format_simple_uuid_like(value);

    let random_canonical = fast_uuid_like()?;
    let random_compact = fast_simple_uuid_like()?;

    println!("{canonical} {compact} {random_canonical} {random_compact}");
    Ok(())
}
```

## Algorithm Notes

`QubitSnowflakeGenerator` is the default Snowflake-style generator for Qubit
Rust services. It uses a fixed high-bit header:

```text
[mode:1][precision:1][timestamp][host:9][sequence]
```

The field widths are:

| Field | Width | Description |
| --- | --- | --- |
| `mode` | 1 bit | Encodes the ID ordering mode: sequential or spread. |
| `precision` | 1 bit | Encodes timestamp precision: millisecond or second. |
| `timestamp` | 41 bits in millisecond precision; 31 bits in second precision | Number of elapsed time slices since the configured epoch. |
| `host` | 9 bits | Host identifier in `0..=511`. |
| `sequence` | 12 bits in millisecond precision; 22 bits in second precision | Incrementing sequence inside the same time slice. |

The fixed `mode` and `precision` positions make those header fields readable
without knowing the timestamp and sequence widths first.

This layout prioritizes a self-describing header, so the ID mode and precision
can be identified directly during parsing.

Qubit Spread IDs always set bit 63 and therefore always exceed `i64::MAX`.
Store them as unsigned 64-bit values, decimal strings, or binary data; use
strings when crossing JavaScript-style safe-integer boundaries.

The 64-bit layout reserves neither a sign bit nor a version field. This is an
intentional capacity and throughput trade-off. A future incompatible layout
must use a new explicit type or API rather than silently changing this one.

Decoding an arbitrary `u64` only extracts fields according to the layout. It
does not prove that the value was produced by this generator and is not an
authenticity or format-validation operation.

### Choosing A Snowflake Generator

| Generator | Strengths | Tradeoffs |
| --- | --- | --- |
| `QubitSnowflakeGenerator` | Encodes `mode` and `precision` in fixed high bits, so parsers can identify layout metadata directly; supports millisecond and second precision, with the default second precision providing a larger per-host sequence space; supports sequential and spread modes; tolerates small clock rollbacks by default. | Uses the Qubit fixed-header layout; the host field is 9 bits, allowing up to 512 host identifiers. |
| `SnowflakeGenerator` | Uses the classic 41-bit millisecond time, 10-bit node, and 12-bit sequence layout; simple and familiar when a traditional Snowflake shape is required. | Fixed layout with no encoded mode or precision; clock rollback returns an error immediately; no spread mode. |
| `SonyflakeGenerator` | Uses a 63-bit ID with 10 ms time units and a 16-bit machine field by default, which fits deployments that need more machine identifiers; sequence and machine bits are configurable. | The default 8-bit sequence per time slice has lower per-machine burst throughput than millisecond Snowflake layouts; 10 ms time units provide coarser ordering. |

For most new services, prefer `QubitSnowflakeGenerator`: it still produces a
compact `u64` numeric ID while keeping layout metadata in fixed high bits, which
makes parsing, debugging, and future evolution more direct. Choose
`SnowflakeGenerator` when the traditional 41/10/12 layout is required, and
choose `SonyflakeGenerator` when machine ID space matters more than per-machine
burst throughput.

### MicaUuidLikeGenerator

`MicaUuidLikeGenerator` is only a random number generator that mimics the
canonical UUID text shape. It uses 128 random bits and formats them as lowercase
UUID-like text. It does not rewrite RFC UUID version or variant bits, so it
should not be treated as a standards-compliant UUID v4 generator.

The UUID-like formatter follows Mica's fast UUID helper and
[`formatUnsignedLong`](https://github.com/lets-mica/mica/blob/master/mica-core/src/main/java/net/dreamlu/mica/core/utils/StringUtil.java#L348)
formatter from
[`StringUtil`](https://github.com/lets-mica/mica/blob/master/mica-core/src/main/java/net/dreamlu/mica/core/utils/StringUtil.java#L335).
Mica's UUID benchmark notes are available in the
[mica-jmh wiki](https://github.com/lets-mica/mica-jmh/wiki/uuid).

## UUID Comparison Benchmark

The fixed-workload benchmark compares Mica UUID-like value generation,
hyphenated strings, and simple strings with the corresponding standard UUID v4
operations. It reports min/median/max throughput without enforcing a
machine-dependent performance threshold:

```text
cargo bench --no-default-features --features uuid --bench uuid_comparison
```

The two generators have different semantics: `MicaUuidLikeGenerator` preserves
all 128 random bits, while UUID v4 sets the standard version and variant bits.
Use the benchmark as local performance evidence, not as a compatibility claim.

## Project Scope

- This crate focuses on local ID generation, not distributed node discovery.
- Qubit Snowflake can wait within its configured rollback tolerance; classic
  Snowflake and Sonyflake return an error for any observed rollback.
- `QubitSnowflakeGenerator` uses its own fixed-header Snowflake layout.
- `SnowflakeGenerator` and `SonyflakeGenerator` are available for services that
  intentionally choose those layouts.

## Contributing

Issues and pull requests are welcome.

Please keep contributions focused and easy to review:

- open an issue for bug reports, design questions, or larger feature proposals
- keep pull requests scoped to one behavior change, fix, or documentation update
- run `./ci-check.sh` before submitting changes
- include tests when changing runtime behavior
- update the README when public API behavior changes

By contributing to this project, you agree that your contribution will be
licensed under the same license as the project.

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

# Qubit ID

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-id` provides object-safe, IoC-friendly synchronous and asynchronous ID
generators for applications that need a numeric ID boundary. It includes three
Snowflake-family layouts and a UUID v4 generator, with typed `Id` values,
structured errors, deterministic clock injection, and blocking, non-blocking,
and asynchronous allocation APIs.

## A practical choice

Suppose an order service needs IDs from several processes and must decide
whether IDs should remain compatible with signed 64-bit database keys, expose a
large machine namespace, or support a configurable layout. Choose the Snowflake
generator from the table below, then read the [user guide](doc/user_guide.md)
for the deployment checklist and configuration trade-offs.

| Generator | Layout and time unit | Nodes | Theoretical throughput per node | Time range | Choose it when |
| --- | --- | ---: | ---: | ---: | --- |
| `ClassicalSnowflakeGenerator` | `1 + 41 ms + 10 node + 12 sequence` | 1,024 | 4,096/ms, about 4.096 million/s | About 69.7 years | You want the simplest, traditional, signed-63-bit layout. |
| `SnowflakeGenerator` | `1 mode + 1 precision + 41 ms + 9 host + 12 sequence` | 512 | 4,096/ms, about 4.096 million/s | About 69.7 years | You want a self-describing Qubit layout with millisecond precision. |
| `SnowflakeGenerator` | `1 mode + 1 precision + 31 s + 9 host + 22 sequence` | 512 | 4,194,304/s, about 4.194 million/s | About 68.1 years | You can use second precision and need a large per-second burst budget. |
| `SonyflakeGenerator` (default layout) | `1 + 39 time + 8 sequence + 16 machine`, 10 ms/unit | 65,536 | 256/10 ms, about 25,600/s | About 174.8 years | You need a larger machine namespace, a longer lifetime, or configurable bit widths. |

The throughput figures are field-capacity limits, not benchmark guarantees. An
ideal aggregate limit is the per-node figure multiplied by the node count; real
throughput also depends on contention, clock progress, and retry waits. Sonyflake
can trade time range, sequence capacity, and machine count by changing its
field widths. Qubit `Spread` uses the same capacity as its selected precision,
but reverses the timestamp bits to reduce direct numeric time correlation; it is
reversible obfuscation, not encryption.

## Highlights

- `IdGenerator`, `TryIdGenerator`, and `AsyncIdGenerator` independently
  provide blocking, non-blocking, and asynchronous allocation capabilities.
  Each trait uses generic `Output` and `Error` parameters, defaulting to `Id`
  and `IdGenerationError`.
- Each Snowflake type implements all three allocation capabilities and shares
  one allocation state across its synchronous and asynchronous call paths.
- Builders accept `Arc<dyn WallClock>` and `Arc<dyn Timer>` from
  [`qubit-clock`](https://crates.io/crates/qubit-clock), making clock rollback,
  sequence rollover, and retry behavior deterministic in tests.
- UUID output is the standards-compliant `uuid::Uuid` version 4 type.
- The default feature set enables only `qubit-snowflake`.

## Installation

The default dependency enables Qubit Snowflake only:

```toml
[dependencies]
qubit-id = "0.5"
```

Select another algorithm independently:

```toml
qubit-id = { version = "0.5", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.5", default-features = false, features = ["sonyflake"] }
qubit-id = { version = "0.5", default-features = false, features = ["uuid"] }
qubit-id = { version = "0.5", default-features = false, features = ["serde"] }

# UUID applications should depend on the upstream type directly.
uuid = { version = "1", features = ["v4"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Asynchronous ID generation is runtime-neutral. Applications that need a Tokio
timer enable the corresponding feature directly on their `qubit-clock`
dependency.

## Quick start

The following service-local generator uses host `7` and returns a typed `Id`:
`SnowflakeGenerator::new` uses sequential mode and second precision by default;
choose `TimestampPrecision::Millisecond` explicitly when millisecond timestamp
resolution is required.

```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn IdGenerator<Id>> =
        Arc::new(SnowflakeGenerator::new(7)?);
    let id = generator.generate()?;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

Every Snowflake generator exposes `try_generate()`, `generate()`, and
`generate_async()`. The concrete asynchronous method has an unboxed outer Future;
a retry may still create the timer's internal Future. For synchronous injection,
use `Arc<dyn IdGenerator<Id>>`. Use
`Arc<dyn AsyncIdGenerator<Id>>` when an
asynchronous object-safe injection boundary is required.

## Snowflake generators

| Feature | Generator type | Layout type |
| --- | --- | --- |
| `qubit-snowflake` | `SnowflakeGenerator` | `SnowflakeLayout` |
| `classic-snowflake` | `ClassicalSnowflakeGenerator` | `ClassicalSnowflakeLayout` |
| `sonyflake` | `SonyflakeGenerator` | `SonyflakeLayout` |

The three layouts use typed `Id` values at their primary boundaries. Use raw
methods only when interoperating with a protocol, database bit pattern, or
another `u64` API:

```rust
use qubit_id::ClassicalSnowflakeLayout;

fn main() -> Result<(), qubit_id::IdGenerationError> {
    let layout = ClassicalSnowflakeLayout::new(7)?;
    let id = layout.compose(42, 3)?;
    let parts = ClassicalSnowflakeLayout::decode(id);
    let raw = layout.compose_raw(42, 3)?;
    let raw_parts = ClassicalSnowflakeLayout::decode_raw(raw);
    assert_eq!(parts, raw_parts);
    Ok(())
}
```

“Classic Snowflake” describes the 41/10/12-bit layout, not a universal epoch.
`ClassicalSnowflakeGenerator` defaults to `2018-12-02T00:00:00Z`, the same epoch
used by Qubit Snowflake. Set the builder's `epoch(...)` when interoperating with
an existing ID namespace that uses a different timestamp origin.

The detailed comparison, selection rules, error handling, and deployment
limits are in the [English user guide](doc/user_guide.md) and
[中文用户手册](doc/user_guide.zh_CN.md).

## IoC contracts and runtime behavior

Each generator uses `Output` and `Error` generic parameters, defaulting to `Id`
and `IdGenerationError`. Third-party generators can provide their own error
type by specifying the parameters explicitly. Select `IdGenerator` for
caller-transparent waits,
`TryIdGenerator` when the caller owns retry scheduling, or `AsyncIdGenerator`
for an object-safe asynchronous boundary.

An object-safe synchronous dependency can use
`Arc<dyn IdGenerator<Id>>`. A UUID
dependency can use
`Arc<dyn IdGenerator<Uuid>>`.

All builders expose `epoch(...)`, `max_clock_skew(...)`, and
`restart_policy(...)`. `RestartPolicy::Immediate` is the default for all Snowflake builders.
`try_generate()`, `generate()`, and `generate_async()` share
one allocation state; cloning a generator, when supported, also shares that
state.

All three Snowflake generators also expose `layout()`, `epoch()`,
`expires_at()`, `max_clock_skew()`, and `compose_at(time, sequence)`.

`expires_at()` returns the exclusive expiration boundary. Builders return
`IdGenerationError::EpochAhead` when the epoch is later than the injected wall
clock. At construction, builders return `IdGenerationError::GeneratorExpired` when `now >= expires_at`.
Clock rollback beyond the configured tolerance returns
`IdGenerationError::ClockMovedBackwards`; timer failures return
`IdGenerationError::WaitFailed`. These are structured errors, not panic-based
control flow. UUID generation returns `IdGenerationError::RandomSourceFailed`
if the operating system cannot provide random bytes.

Applications must assign an exclusive host, node, or machine identifier to
every concurrently active generator in the same namespace. This crate does not
persist allocation state or provide a distributed lease.

## Storage and transport

| Generator mode | Recommended representation |
| --- | --- |
| Sequential Qubit, classic Snowflake, Sonyflake | `Id`, `u64`, or decimal text; checked `i64` conversion when the selected layout remains within range |
| Spread Qubit | `Id`, unsigned decimal text, or 8-byte binary data |
| UUID v4 | `uuid::Uuid`, 16-byte binary data, or canonical UUID text |

`IdMode::Spread` always sets bit 63, so its IDs exceed `i64::MAX`. Do not cast
those IDs to a signed database key. Use decimal strings when IDs cross
JavaScript or JSON boundaries. With the optional `serde` feature, human-readable
`Id` serialization uses decimal strings and compact serialization uses `u64`.

## Deterministic time injection

Derive the wall clock and timer from the same `ManualMonotonicClock` in tests:

```text
let clock = ManualMonotonicClock::new_shared();
let generator = SnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build()?;
```

Manual timer observers let tests wait until a deadline is registered before
advancing logical time, avoiding real sleeps and scheduling guesses.

## UUID v4

`UuidV4Generator` returns `uuid::Uuid`. Import `Uuid` directly from the `uuid`
dependency to use its parsing, formatting, version, and byte APIs. Async
applications should place this synchronous call behind the blocking boundary
provided by their chosen runtime:

```rust
use std::sync::Arc;
use qubit_id::{IdGenerationError, IdGenerator, UuidV4Generator};
use uuid::Uuid;

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn IdGenerator<Uuid>> =
        Arc::new(UuidV4Generator::new());
    let uuid = generator.generate()?;
    assert_eq!(uuid.to_string().len(), 36);
    Ok(())
}
```

## Features and benchmarks

```bash
# Qubit concrete, dynamic-dispatch, and asynchronous call paths
cargo bench --bench qubit_snowflake_throughput

# qubit-id UUID wrappers versus direct uuid crate calls
cargo bench --no-default-features --features uuid --bench uuid_comparison
```

## Further reading

- [English user guide](doc/user_guide.md)
- [中文用户手册](doc/user_guide.zh_CN.md)
- [中文 README](README.zh_CN.md)
- [API documentation](https://docs.rs/qubit-id)

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-id](https://github.com/qubit-ltd/rs-id)

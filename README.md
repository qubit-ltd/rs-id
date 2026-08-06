# Qubit ID

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-id` provides object-safe, IoC-friendly synchronous and asynchronous ID
generators. Snowflake generators return `Id`, while UUID generators return
`uuid::Uuid`; applications can
use their numeric values or their display forms at the storage and transport
boundary.

## Highlights

- `TryIdGenerator`, `IdGenerator`, and `AsyncIdGenerator` separate non-blocking, blocking, and
  asynchronous allocation contracts.
- Each Snowflake type implements all three contracts and shares one allocation
  state across its synchronous and asynchronous call paths.
- Builders accept `Arc<dyn WallClock>` and `Arc<dyn Timer>` from
  [`qubit-clock`](https://crates.io/crates/qubit-clock).
- UUID output is the standards-compliant `uuid::Uuid` version 4 type, with
  canonical hyphenated text and the full `uuid` crate API.
- The default feature set enables only `qubit-snowflake`.

## Installation

The default dependency enables Qubit Snowflake only:

```toml
[dependencies]
qubit-id = "0.3"
```

Select another algorithm independently:

```toml
qubit-id = { version = "0.3", default-features = false, features = ["classic-snowflake"] }
qubit-id = { version = "0.3", default-features = false, features = ["sonyflake"] }
qubit-id = { version = "0.3", default-features = false, features = ["uuid"] }
qubit-id = { version = "0.3", default-features = false, features = ["serde"] }

# UUID applications should depend on the upstream type directly.
uuid = { version = "1", features = ["v4"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Asynchronous ID generation is runtime-neutral. Applications that need the
Tokio timer enable the corresponding feature directly on their `qubit-clock`
dependency.

## IoC contracts

Each generator declares its `Output` and `Error` as associated types. Built-in
generators use `IdGenerationError`; third-party generators can provide their
own concrete error type.

Use `Arc<dyn IdGenerator<Output = Id, Error = IdGenerationError>>` for a
synchronous Snowflake dependency:

```rust
use std::sync::Arc;
use qubit_id::{Id, IdGenerationError, IdGenerator, SnowflakeGenerator};

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn IdGenerator<Output = Id, Error = IdGenerationError>> =
        Arc::new(SnowflakeGenerator::new(7)?);
    let id = generator.generate()?;
    assert_ne!(id.value(), 0);
    Ok(())
}
```

Every Snowflake generator exposes blocking, non-blocking, and asynchronous
methods. The concrete asynchronous method has an unboxed outer Future; a retry
may still create the timer's internal Future:

```rust
use qubit_id::{Id, IdGenerationError, SnowflakeGenerator};

async fn allocate_concrete(
    generator: &SnowflakeGenerator,
) -> Result<Id, IdGenerationError> {
    generator.generate_async().await
}
```

Use `Arc<dyn TryIdGenerator<Output = Id, Error = IdGenerationError>>` when the
caller owns retry scheduling:

```rust
use std::sync::Arc;
use qubit_id::{GenerationAttempt, Id, IdGenerationError, SnowflakeGenerator, TryIdGenerator};

fn allocate(
    generator: &dyn TryIdGenerator<Output = Id, Error = IdGenerationError>,
) -> Id {
    match generator.try_generate().expect("allocation should be valid") {
        GenerationAttempt::Generated(id) => id,
        GenerationAttempt::RetryAfter { delay: _ } => Id::from(0),
    }
}

let generator: Arc<dyn TryIdGenerator<Output = Id, Error = IdGenerationError>> =
    Arc::new(SnowflakeGenerator::new(7).expect("valid host"));
let _ = allocate(generator.as_ref());
```

Use `Arc<dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>>` when an asynchronous object-safe injection
boundary is required. Dynamic dispatch returns a boxed Future and does not require Tokio:

```rust
use std::sync::Arc;
use qubit_id::{
    AsyncIdGenerator, Id, IdGenerationError, SnowflakeGenerator,
};

async fn allocate(
    generator: &dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>,
) -> Result<Id, IdGenerationError> {
    generator.generate_async().await
}

fn main() -> Result<(), IdGenerationError> {
    let generator: Arc<dyn AsyncIdGenerator<Output = Id, Error = IdGenerationError>> =
        Arc::new(SnowflakeGenerator::new(7)?);
    let _injected = generator;
    Ok(())
}
```

A test mock only needs to implement the relevant trait for its local type.

## Snowflake generators

| Feature | Generator type | Native output |
| --- | --- | --- |
| `qubit-snowflake` | `SnowflakeGenerator` | `Id` |
| `classic-snowflake` | `ClassicalSnowflakeGenerator` | `Id` |
| `sonyflake` | `SonyflakeGenerator` | `Id` |

The three public Snowflake layouts use typed `Id` values at their primary
boundaries. Use the explicit raw methods only when interoperating with a
protocol, database bit pattern, or another `u64` API:

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
used by Qubit Snowflake. Set the builder's `epoch(...)` when interoperating with an
existing ID namespace that uses a different timestamp origin.

Each builder has one `build()` method. The resulting generator shares one
allocation state across `try_generate()`, `generate()`, and `generate_async()`;
cloning a generator, when supported, also shares that state.

### Storage and transport compatibility

| Output | Compatible storage and transport |
| --- | --- |
| Sequential Qubit, classic Snowflake, Sonyflake | `Id`, `u64`, or decimal text; checked `i64` conversion when the selected layout remains within its range |
| Spread Qubit | `Id`, unsigned decimal text, or 8-byte binary data |
| UUID v4 | `uuid::Uuid`, 16-byte binary data, or canonical UUID text |

`IdMode::Spread` always sets bit 63, so its IDs exceed `i64::MAX`.
Do not cast those IDs to a signed database key. Use decimal strings when IDs
cross JavaScript or JSON boundaries.

## Deterministic time injection

Derive the wall clock and timer from the same `ManualMonotonicClock` in tests.
The same pattern works with `StdWallClock`, `StdMonotonicClock`, and a Tokio
timer enabled directly through `qubit-clock`:

```text
let clock = ManualMonotonicClock::new_shared();
let generator = SnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build()?;
```

Manual timer observers let tests wait until a deadline is registered before
advancing logical time, avoiding real sleeps and scheduling guesses.

A Tokio timer retains its target runtime handle, so `generate_async()` can be
polled from another runtime or execution context. The target `Runtime` must
remain alive and driven. `generate()` blocks while waiting, so its timer backend
must progress independently of the caller thread; use `try_generate()` when the
caller must own scheduling.

## Domain values and text output

Snowflake generators return `Id`. It is a transparent `u64` value with decimal
`Display` formatting, so callers can choose the representation at the
application boundary:

```rust
use qubit_id::{IdGenerationError, SnowflakeGenerator};

fn main() -> Result<(), IdGenerationError> {
    let id = SnowflakeGenerator::new(7)?.generate()?;
    let numeric: u64 = id.into();
    let text = id.to_string();
    assert_eq!(text, numeric.to_string());
    Ok(())
}
```

`UuidV4Generator` returns `uuid::Uuid`. Import `Uuid` directly from the `uuid`
dependency to use its parsing, formatting, version, and byte APIs. UUID
generation returns `IdGenerationError::RandomSourceFailed`
if the operating system cannot provide
random bytes. Async applications should place the synchronous call behind the
blocking boundary supplied by their chosen runtime:

```rust
use qubit_id::{IdGenerationError, UuidV4Generator};
use uuid::Uuid;

fn main() -> Result<(), IdGenerationError> {
    let uuid = UuidV4Generator::new().generate()?;
    let numeric = Uuid::as_u128(&uuid);
    let text = uuid.to_string();
    assert_ne!(numeric, 0);
    assert_eq!(text.len(), 36);
    Ok(())
}
```

### Serialization formats

Enable the optional `serde` feature when serializing `Id`:

| Format | `Id` representation | Accepted input |
| --- | --- | --- |
| Human-readable (JSON) | Decimal string, for example `"42"` | Decimal string only |
| Compact/binary | Unsigned 64-bit integer | `u64` only |

JSON numbers are rejected even when they are small enough for JavaScript's
safe-integer range, because a JSON number can cross an IEEE-754 boundary and
silently lose precision for a 64-bit ID. UUID serialization follows the
native `uuid` crate contract: canonical text for human-readable formats and
16 bytes for compact formats.

## Lifetime, clocks, and deployment identity

`expires_at()` returns the exclusive expiration boundary. A builder samples
its injected wall clock; builders return `IdGenerationError::GeneratorExpired` when `now >= expires_at`,
because that configuration cannot serve IDs. A live generator that later
reaches the same boundary returns the same error.

A small Qubit clock rollback can wait up to `max_clock_skew`; a larger rollback
returns `IdGenerationError::ClockMovedBackwards`. Classic Snowflake and Sonyflake reject
any rollback. Timer registration or blocking-adapter failures are returned as
`IdGenerationError::WaitFailed` with the original `TimeError` source.

Applications must assign an exclusive host, node, or machine identifier to
every concurrently active generator in the same namespace. This crate does not
persist allocation state or provide a distributed lease.

`RestartPolicy::WaitNextSlice` is the default for all Snowflake builders. It
delays the first allocation until a later time slice, reducing same-slice reuse
after a stopped instance is replaced. `RestartPolicy::Immediate` is available
for deployments that externally guarantee restart separation. Neither policy
coordinates concurrently active generators or replaces persistent allocation
state and an exclusive distributed identity lease.

## Features and benchmarks

```bash
# Qubit concrete, dynamic-dispatch, and asynchronous call paths
cargo bench --bench qubit_snowflake_throughput

# qubit-id UUID wrappers versus direct uuid crate calls
cargo bench --no-default-features --features uuid --bench uuid_comparison
```

Benchmarks report measurements only; normal tests do not assert unstable
performance thresholds.

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

# Qubit ID

[![Rust CI](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-id/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-id/coverage-badge.json)](https://qubit-ltd.github.io/rs-id/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

`qubit-id` provides object-safe, IoC-friendly synchronous and asynchronous ID
generators. Applications choose a natural output boundary (`u64`, `u128`, or
`String`) and can replace the production implementation with a local mock in
tests.

## Highlights

- `TryIdGenerator<T, E = IdError>`, `IdGenerator<T, E = IdError>`, and
  `AsyncIdGenerator<T, E = IdError>` separate non-blocking, blocking, and
  asynchronous allocation contracts.
- Each Snowflake type implements all three contracts and shares one allocation
  state across its synchronous and asynchronous call paths.
- Builders accept `Arc<dyn WallClock>` and `Arc<dyn Timer>` from
  [`qubit-clock`](https://crates.io/crates/qubit-clock).
- UUID output is a standards-compliant version 4 UUID, available as `u128` or
  canonical hyphenated text.
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
```

Asynchronous ID generation is runtime-neutral. Applications that need the
Tokio timer enable the corresponding feature directly on their `qubit-clock`
dependency.

## IoC contracts

The error parameter defaults to `IdError`, so built-in generators keep the
short `IdGenerator<T>` spelling. Third-party implementations can retain a
provider-specific error type with `IdGenerator<T, E>`.

Use `Arc<dyn IdGenerator<u64>>` for a synchronous numeric dependency:

```rust
use std::sync::Arc;
use qubit_id::{IdError, IdGenerator, QubitSnowflakeGenerator};

fn main() -> Result<(), IdError> {
    let generator: Arc<dyn IdGenerator<u64>> =
        Arc::new(QubitSnowflakeGenerator::new(7)?);
    let id = generator.generate()?;
    assert_ne!(id, 0);
    Ok(())
}
```

Every Snowflake generator exposes blocking, non-blocking, and asynchronous
methods. The concrete asynchronous method has an unboxed outer Future; a retry
may still create the timer's internal Future:

```rust
use qubit_id::{QubitSnowflakeGenerator, IdError};

async fn allocate_concrete(
    generator: &QubitSnowflakeGenerator,
) -> Result<u64, IdError> {
    generator.generate_async().await
}
```

Use `Arc<dyn TryIdGenerator<u64>>` when the caller owns retry scheduling:

```rust
use std::sync::Arc;
use qubit_id::{GenerationAttempt, QubitSnowflakeGenerator, TryIdGenerator};

fn allocate(generator: &dyn TryIdGenerator<u64>) -> u64 {
    match generator.try_generate().expect("allocation should be valid") {
        GenerationAttempt::Generated(id) => id,
        GenerationAttempt::RetryAfter { delay: _ } => 0,
    }
}

let generator: Arc<dyn TryIdGenerator<u64>> =
    Arc::new(QubitSnowflakeGenerator::new(7).expect("valid host"));
let _ = allocate(generator.as_ref());
```

Use `Arc<dyn AsyncIdGenerator<u64>>` when an asynchronous object-safe injection
boundary is required. Dynamic dispatch returns a boxed Future and does not require Tokio:

```rust
use std::sync::Arc;
use qubit_id::{
    AsyncIdGenerator, QubitSnowflakeGenerator, IdError,
};

async fn allocate(
    generator: &dyn AsyncIdGenerator<u64>,
) -> Result<u64, IdError> {
    generator.generate_async().await
}

fn main() -> Result<(), IdError> {
    let generator: Arc<dyn AsyncIdGenerator<u64>> =
        Arc::new(QubitSnowflakeGenerator::new(7)?);
    let _injected = generator;
    Ok(())
}
```

A test mock only needs to implement the relevant trait for its local type.

## Snowflake generators

| Feature | Generator type | Native output |
| --- | --- | --- |
| `qubit-snowflake` | `QubitSnowflakeGenerator` | `u64` |
| `classic-snowflake` | `SnowflakeGenerator` | `u64` |
| `sonyflake` | `SonyflakeGenerator` | `u64` |

“Classic Snowflake” describes the 41/10/12-bit layout, not a universal epoch.
`SnowflakeGenerator` defaults to `2018-12-02T00:00:00Z`, the same epoch used by
Qubit Snowflake. Set the builder's `epoch(...)` when interoperating with an
existing ID namespace that uses a different timestamp origin.

Each builder has one `build()` method. The resulting generator shares one
allocation state across `try_generate()`, `generate()`, and `generate_async()`;
cloning a generator, when supported, also shares that state.

### Storage and transport compatibility

| Output | Compatible storage and transport |
| --- | --- |
| Sequential Qubit, classic Snowflake, Sonyflake | `u64`; checked `i64` conversion when the selected layout remains within its range |
| Spread Qubit | `u64`, unsigned decimal text, or 8-byte binary data |
| UUID v4 | `u128`, 16-byte binary data, or canonical UUID text |

`IdMode::Spread` always sets bit 63, so its IDs exceed `i64::MAX`.
Do not cast those IDs to a signed database key. Use decimal strings when IDs
cross JavaScript or JSON boundaries.

## Deterministic time injection

Derive the wall clock and timer from the same `ManualMonotonicClock` in tests.
The same pattern works with `StdWallClock`, `StdMonotonicClock`, and a Tokio
timer enabled directly through `qubit-clock`:

```text
let clock = ManualMonotonicClock::new_shared();
let generator = QubitSnowflakeGenerator::builder(7)
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

## String and UUID outputs

Wrap any Snowflake `u64` generator when the IoC
boundary needs decimal text:

```rust
use std::sync::Arc;
use qubit_id::{
    IdError, IdGenerator, QubitSnowflakeGenerator,
    SnowflakeStringGenerator,
};

fn main() -> Result<(), IdError> {
    let numeric = QubitSnowflakeGenerator::new(7)?;
    let generator: Arc<dyn IdGenerator<String>> =
        Arc::new(SnowflakeStringGenerator::new(numeric));
    let value = generator.generate()?;
    assert!(value.parse::<u64>().is_ok());
    Ok(())
}
```

`UuidV4Generator` returns `u128`; `UuidV4StringGenerator` returns canonical
hyphenated text. Both implement the synchronous contract.
UUID generation returns `IdError::RandomSourceFailed` if the operating system
cannot provide random bytes. Async applications should place the synchronous
call behind the blocking boundary supplied by their chosen runtime:

```rust
use qubit_id::{
    IdError, IdGenerator, UuidV4Generator, UuidV4StringGenerator,
};

fn main() -> Result<(), IdError> {
    let numeric = UuidV4Generator::new().generate()?;
    let text = UuidV4StringGenerator::new().generate()?;
    assert_ne!(numeric, 0);
    assert_eq!(text.len(), 36);
    Ok(())
}
```

## Lifetime, clocks, and deployment identity

`expires_at()` returns the exclusive expiration boundary. A builder samples
its injected wall clock; builders return `IdError::GeneratorExpired` when `now >= expires_at`,
because that configuration cannot serve IDs. A live generator that later
reaches the same boundary returns the same error.

A small Qubit clock rollback can wait up to `max_clock_skew`; a larger rollback
returns `IdError::ClockMovedBackwards`. Classic Snowflake and Sonyflake reject
any rollback. Timer registration or blocking-adapter failures are returned as
`IdError::WaitFailed` with the original `TimeError` source.

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

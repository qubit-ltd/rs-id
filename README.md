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

- `IdGenerator<T, E = IdError>` and
  `AsyncIdGenerator<T, E = IdError>` support provider-specific error types and
  use `&self`, with state changes synchronized inside each implementation.
- Qubit Snowflake, classic Snowflake, and Sonyflake share one allocation core
  while using separate blocking and asynchronous wait drivers.
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

Concrete asynchronous generators expose an allocation-free inherent Future:

```rust
use qubit_id::{AsyncQubitSnowflakeGenerator, IdError};

async fn allocate_concrete(
    generator: &AsyncQubitSnowflakeGenerator,
) -> Result<u64, IdError> {
    generator.generate_async().await
}
```

Use `Arc<dyn AsyncIdGenerator<u64>>` when an object-safe injection boundary is
required. Dynamic dispatch returns a boxed Future and does not require Tokio:

```rust
use std::sync::Arc;
use qubit_id::{
    AsyncIdGenerator, AsyncQubitSnowflakeGenerator, IdError,
};

async fn allocate(
    generator: &dyn AsyncIdGenerator<u64>,
) -> Result<u64, IdError> {
    generator.generate_async().await
}

fn main() -> Result<(), IdError> {
    let generator: Arc<dyn AsyncIdGenerator<u64>> =
        Arc::new(AsyncQubitSnowflakeGenerator::new(7)?);
    let _injected = generator;
    Ok(())
}
```

A test mock only needs to implement the relevant trait for its local type.

## Snowflake generators

| Feature | Synchronous type | Asynchronous type | Native output |
| --- | --- | --- | --- |
| `qubit-snowflake` | `QubitSnowflakeGenerator` | `AsyncQubitSnowflakeGenerator` | `u64` |
| `classic-snowflake` | `SnowflakeGenerator` | `AsyncSnowflakeGenerator` | `u64` |
| `sonyflake` | `SonyflakeGenerator` | `AsyncSonyflakeGenerator` | `u64` |

Each builder has `build()` and `build_async()`. Both consume the same layout,
epoch/start time, restart policy, wall clock, and timer configuration.

## Deterministic time injection

Derive the wall clock and timer from the same `ManualMonotonicClock` in tests.
The same pattern works with `StdWallClock`, `StdMonotonicClock`, and a Tokio
timer enabled directly through `qubit-clock`:

```text
let clock = ManualMonotonicClock::new_shared();
let generator = QubitSnowflakeGenerator::builder(7)
    .wall_clock(clock.new_wall_clock(initial_time))
    .timer(clock.new_timer())
    .build_async()?;
```

Manual timer observers let tests wait until a deadline is registered before
advancing logical time, avoiding real sleeps and scheduling guesses.

## String and UUID outputs

Wrap any synchronous or asynchronous Snowflake `u64` generator when the IoC
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
hyphenated text. Both implement the synchronous and asynchronous contracts and
return `IdError::RandomSourceFailed` if the operating system cannot provide
random bytes:

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

UUID generation panics if the operating-system random source is unavailable.

## Lifetime, clocks, and deployment identity

`expires_at()` returns the exclusive expiration boundary. A builder samples
its injected wall clock and panics when `now >= expires_at`, because that
configuration cannot serve IDs. A live generator that later reaches the same
boundary returns `IdError::GeneratorExpired`.

A small Qubit clock rollback can wait up to `max_clock_skew`; a larger rollback
returns `IdError::ClockMovedBackwards`. Classic Snowflake and Sonyflake reject
any rollback. Timer registration or blocking-adapter failures are returned as
`IdError::WaitFailed` with the original `TimeError` source.

Applications must assign an exclusive host, node, or machine identifier to
every concurrently active generator in the same namespace. This crate does not
persist allocation state or provide a distributed lease.

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

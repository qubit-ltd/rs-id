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

It includes one common `IdGenerator<T>` trait plus generators for
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

```toml
[dependencies]
qubit-id = "0.2.1"
```

## Quick Start

```rust
use qubit_id::{IdGenerator, MicaUuidLikeGenerator, QubitSnowflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snowflake = QubitSnowflakeGenerator::new(1)?;
    let id: u64 = snowflake.next_id()?;
    let id_text = snowflake.next_string()?;

    let uuid_like = MicaUuidLikeGenerator::new();
    let uuid_like_value: u128 = uuid_like.next_id()?;
    let uuid_like_text = uuid_like.next_string()?;

    println!("{id} {id_text} {uuid_like_value} {uuid_like_text}");
    Ok(())
}
```

## Core API At A Glance

| Type | Purpose |
| --- | --- |
| `IdGenerator<T>` | Common trait for typed ID generation and string formatting. |
| `QubitSnowflakeGenerator` | Qubit fixed-header Snowflake generator. |
| `QubitSnowflakeBuilder` | Builds and inspects Qubit Snowflake bit layouts. |
| `SnowflakeGenerator` | Classic 41-bit time, 10-bit node, 12-bit sequence Snowflake generator. |
| `SonyflakeGenerator` | Sonyflake-style generator with configurable sequence and machine bits. |
| `MicaUuidLikeGenerator` | Mica-style random 128-bit UUID-like generator. |
| `fast_uuid_like` | Generates canonical lowercase UUID-like text. |
| `fast_simple_uuid_like` | Generates compact lowercase 32-hex UUID-like text. |

## Generator Examples

### QubitSnowflakeGenerator

Use `QubitSnowflakeGenerator` for the Qubit fixed-header Snowflake layout. The
default constructor uses sequential mode, second precision, and the default
Qubit epoch.

```rust
use qubit_id::{IdGenerator, QubitSnowflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The argument is the 9-bit host ID encoded into generated IDs.
    // It must be in the range 0..=511.
    let generator = QubitSnowflakeGenerator::new(42)?;

    let id = generator.next_id()?;
    let id_text = generator.next_string()?;

    let builder = generator.builder();
    assert_eq!(builder.extract_host(id), 42);

    println!("{id} {id_text}");
    Ok(())
}
```

Configure the Qubit layout explicitly when you need spread mode or millisecond
precision.

```rust
use std::time::{Duration, UNIX_EPOCH};

use qubit_id::{
    IdGenerator, IdMode, QubitSnowflakeGenerator, TimestampPrecision,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = QubitSnowflakeGenerator::with_options(
        IdMode::Spread,
        TimestampPrecision::Millisecond,
        7,
        UNIX_EPOCH + Duration::from_millis(1_543_708_800_000),
    )?;

    let id = generator.next_id()?;
    let builder = generator.builder();

    assert_eq!(builder.extract_mode(id), IdMode::Spread);
    assert_eq!(builder.extract_precision(id), TimestampPrecision::Millisecond);
    assert_eq!(builder.extract_host(id), 7);

    Ok(())
}
```

### SnowflakeGenerator

Use `SnowflakeGenerator` when you need the classic 41-bit millisecond timestamp,
10-bit node, and 12-bit sequence layout.

```rust
use qubit_id::{IdGenerator, SnowflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SnowflakeGenerator::new(3)?;

    let id = generator.next_id()?;

    assert_eq!(generator.extract_node_id(id), 3);
    println!("{id}");

    Ok(())
}
```

You can also compose and inspect deterministic IDs from known parts.

```rust
use qubit_id::SnowflakeGenerator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SnowflakeGenerator::new(3)?;
    let id = generator.compose(1_000, 5)?;

    assert_eq!(generator.extract_timestamp(id), 1_000);
    assert_eq!(generator.extract_node_id(id), 3);
    assert_eq!(generator.extract_sequence(id), 5);

    Ok(())
}
```

### SonyflakeGenerator

Use `SonyflakeGenerator` when a larger machine ID space matters more than
per-machine burst throughput.

```rust
use qubit_id::{IdGenerator, SonyflakeGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let generator = SonyflakeGenerator::new(65_535)?;

    let id = generator.next_id()?;

    assert_eq!(generator.extract_machine_id(id), 65_535);
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
    let generator = SonyflakeGenerator::with_options(
        15,
        10,
        14,
        Duration::from_millis(1),
        UNIX_EPOCH + Duration::from_secs(1_735_689_600),
    )?;

    let id = generator.next_id()?;

    assert_eq!(generator.bits_sequence(), 10);
    assert_eq!(generator.bits_machine(), 14);
    assert_eq!(generator.extract_machine_id(id), 15);

    Ok(())
}
```

### MicaUuidLikeGenerator And Helpers

Use `MicaUuidLikeGenerator` when you want a random 128-bit value with UUID-like
lowercase text formatting. Use the helper functions when you only need strings.

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

## Project Scope

- This crate focuses on local ID generation, not distributed node discovery.
- Clock rollback is handled by waiting within the configured tolerance and
  returning an explicit error when the skew is too large.
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

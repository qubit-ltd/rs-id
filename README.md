# Qubit ID (`rs-id`)

[![CircleCI](https://circleci.com/gh/qubit-ltd/rs-id.svg?style=shield)](https://circleci.com/gh/qubit-ltd/rs-id)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-id/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-id?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-id.svg?color=blue)](https://crates.io/crates/qubit-id)
[![Docs.rs](https://docs.rs/qubit-id/badge.svg)](https://docs.rs/qubit-id)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Documentation: [API Reference](https://docs.rs/qubit-id)

`qubit-id` provides ID generation utilities for Rust services.

It includes one common `IdGenerator<T>` trait plus generators for
database-friendly Snowflake IDs, Sonyflake-style IDs, and fast UUID-shaped
random identifiers.

## Why Use It

Use `qubit-id` when you need:

- Qubit Snowflake IDs with fixed high-bit mode and precision headers
- classic Snowflake IDs with a compact 64-bit numeric representation
- Sonyflake-style IDs with longer runtime under small sequence pressure
- fast UUID-shaped random strings matching the existing Java helper behavior
- one trait-based API that can return typed IDs and string representations

## Installation

```toml
[dependencies]
qubit-id = "0.1.0"
```

## Quick Start

```rust
use qubit_id::{IdGenerator, QubitSnowflakeGenerator, UuidGenerator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let snowflake = QubitSnowflakeGenerator::new(1)?;
    let id: u64 = snowflake.next_id()?;
    let id_text = snowflake.next_string()?;

    let uuid = UuidGenerator::new();
    let uuid_value: u128 = uuid.next_id()?;
    let uuid_text = uuid.next_string()?;

    println!("{id} {id_text} {uuid_value} {uuid_text}");
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
| `UuidGenerator` | Fast random 128-bit UUID-shaped generator. |
| `fast_uuid` | Generates canonical lowercase UUID text. |
| `fast_simple_uuid` | Generates compact lowercase 32-hex UUID text. |

## Algorithm Notes

`QubitSnowflakeGenerator` is the default Snowflake-style generator for Qubit
Rust services. It uses a fixed high-bit header:

```text
[mode:1][precision:1][timestamp][host:9][sequence]
```

The fixed `mode` and `precision` positions make those header fields readable
without knowing the timestamp and sequence widths first.

This Rust layout intentionally differs from the earlier Java `common-id`
Snowflake layout:

```text
[mode:1][timestamp][precision:1][host:9][sequence]
```

The Java layout is useful historical context, but the Rust Qubit layout now
prioritizes a self-describing header over binary compatibility with the Java
IDs.

`SnowflakeGenerator` is useful when a standard Snowflake layout is preferred:
41 bits of milliseconds, 10 bits of node ID, and 12 bits of sequence.

`SonyflakeGenerator` follows Sonyflake-style tradeoffs: a 63-bit ID using a
larger machine field and 10 ms time units by default. This reduces per-machine
throughput compared with classic Snowflake, but gives a longer useful lifetime
and a larger machine ID space.

`UuidGenerator` intentionally matches the Java fast UUID helper by using 128
random bits and formatting them as lowercase UUID text. It does not rewrite RFC
version or variant bits.

## Project Scope

- This crate focuses on local ID generation, not distributed node discovery.
- Clock rollback is handled by waiting within the configured tolerance and
  returning an explicit error when the skew is too large.
- `QubitSnowflakeGenerator` is not binary-compatible with the earlier Java
  `common-id` Snowflake layout.
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

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Qubit ID
//!
//! IoC-friendly ID generation utilities for Rust services. The crate provides
//! object-safe synchronous [`IdGenerator<T>`](IdGenerator) and asynchronous
//! [`AsyncIdGenerator<T>`](AsyncIdGenerator) contracts, three Snowflake-family
//! algorithms, decimal string adaptation, and standards-compliant UUID v4.
//!
//! ## Cargo features
//!
//! The default feature set contains only `qubit-snowflake`. The
//! `classic-snowflake`, `sonyflake`, and `uuid` algorithms are independently
//! opt-in. The `tokio` feature enables the corresponding `qubit-clock` timer
//! adapter but is not required by asynchronous generators. Disabling every
//! feature leaves the common generator traits and error API available.
//!
//! | Feature | Contents |
//! | --- | --- |
//! | `qubit-snowflake` | Synchronous and asynchronous Qubit Snowflake generators |
//! | `classic-snowflake` | Synchronous and asynchronous classic Snowflake generators |
//! | `sonyflake` | Synchronous and asynchronous Sonyflake generators |
//! | `uuid` | Numeric and canonical-string UUID v4 generators |
//! | `tokio` | `qubit-clock` Tokio timer adapter |
//!
//! ## Allocation and lifetime
//!
//! [`IdGenerator::generate`] may block while a Snowflake generator waits for
//! time to advance. [`AsyncIdGenerator::generate_async`] awaits the injected
//! `qubit_clock::Timer` without blocking an executor. Configure
//! `RestartPolicy::WaitNextSlice` when a fresh instance should skip its first
//! observed logical time slice. This policy does not know the predecessor's
//! allocation watermark, so clock rollback across a restart can still repeat
//! IDs. The default `RestartPolicy::Immediate` can repeat IDs after
//! same-slice allocation state is lost.
//!
//! Every Snowflake layout calculates an exclusive expiration boundary from its
//! time origin, unit, and maximum timestamp. Generators cache that value and
//! expose it through `expires_at()`. Construction panics when the configured
//! wall clock is equal to or later than the boundary. An unrepresentable
//! boundary returns [`IdError::ExpirationTimeOverflow`] instead.
//!
//! This crate does not persist allocation state, coordinate generator
//! identities across processes, reserve a layout version field, or provide an
//! authenticity check for decoded IDs. Applications must assign exclusive
//! host, node, or machine identifiers for concurrently active generators in
//! the same namespace.
//!
//! UUID generator wrapper overhead can be compared with direct `uuid` crate
//! calls by running
//! `cargo bench --no-default-features --features uuid --bench uuid_comparison`.

#![deny(missing_docs)]

mod async_id_generator;
mod id_error;
mod id_generation_future;
mod id_generator;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
pub mod snowflake;
#[cfg(feature = "uuid")]
pub mod uuid;

pub use async_id_generator::AsyncIdGenerator;
pub use id_error::IdError;
pub use id_generation_future::IdGenerationFuture;
pub use id_generator::IdGenerator;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
pub use snowflake::RestartPolicy;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
pub use snowflake::SnowflakeStringGenerator;
#[cfg(feature = "qubit-snowflake")]
pub use snowflake::{
    AsyncQubitSnowflakeGenerator,
    DEFAULT_MAX_CLOCK_SKEW,
    HOST_BITS,
    HOST_MAX,
    HOST_MIN,
    IdMode,
    PRECISION_BITS,
    QubitSnowflakeGenerator,
    QubitSnowflakeGeneratorBuilder,
    QubitSnowflakeLayout,
    QubitSnowflakeParts,
    TimestampPrecision,
};
#[cfg(feature = "classic-snowflake")]
pub use snowflake::{
    AsyncSnowflakeGenerator,
    SnowflakeGenerator,
    SnowflakeGeneratorBuilder,
    SnowflakeLayout,
    SnowflakeParts,
};
#[cfg(feature = "sonyflake")]
pub use snowflake::{
    AsyncSonyflakeGenerator,
    SonyflakeGenerator,
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
    SonyflakeParts,
};
#[cfg(feature = "uuid")]
pub use uuid::{
    UuidV4Generator,
    UuidV4StringGenerator,
};

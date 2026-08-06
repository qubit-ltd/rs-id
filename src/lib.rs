// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
#![cfg_attr(docsrs, feature(doc_cfg))]
//! # Qubit ID
//!
//! IoC-friendly ID generation utilities for Rust services. The crate provides
//! object-safe non-blocking [`TryIdGenerator`], synchronous [`IdGenerator`],
//! and asynchronous [`AsyncIdGenerator`] contracts, three Snowflake-family
//! algorithms, domain ID values, and standards-compliant UUID v4.
//!
//! ## Cargo features
//!
//! The default feature set contains only `qubit-snowflake`. The
//! `classic-snowflake`, `sonyflake`, `uuid`, and `serde` features are
//! independently opt-in. Asynchronous generators are runtime-neutral; enable runtime-specific
//! timers directly on `qubit-clock`. Disabling every feature leaves the common
//! generator traits and error API available.
//!
//! | Feature | Contents |
//! | --- | --- |
//! | `qubit-snowflake` | Synchronous and asynchronous Qubit Snowflake generators |
//! | `classic-snowflake` | Synchronous and asynchronous classic Snowflake generators |
//! | `sonyflake` | Synchronous and asynchronous Sonyflake generators |
//! | `uuid` | UUID v4 generator returning `uuid::Uuid` |
//! | `serde` | Optional `Id` serialization and deserialization |
//!
//! UUID generators intentionally implement only [`IdGenerator`], because the
//! operating-system random source may block. Async applications should choose
//! their runtime-specific blocking boundary explicitly.
//!
//! ## Allocation and lifetime
//!
//! [`TryIdGenerator::try_generate`] never sleeps or awaits and returns a
//! [`GenerationAttempt`] when the caller must schedule a retry. A Snowflake
//! [`IdGenerator::generate`] may block while waiting for time to advance.
//! Concrete generators expose an inherent `generate_async` method whose outer
//! future is unboxed. Calling
//! [`AsyncIdGenerator::generate_async`] through the object-safe trait boxes the
//! future and awaits the injected `qubit_clock::Timer` without blocking an
//! executor. `RestartPolicy::WaitNextSlice` is the default and makes a fresh instance
//! skip its first observed logical time slice. This policy does not know the
//! predecessor's allocation watermark, so clock rollback across a restart can
//! still repeat IDs. `RestartPolicy::Immediate` can repeat IDs after same-slice
//! allocation state is lost and should only be selected when restart separation
//! is guaranteed externally.
//!
//! Every Snowflake layout calculates an exclusive expiration boundary from its
//! time origin, unit, and maximum timestamp. Generators cache that value and
//! expose it through `expires_at()`. Construction returns
//! [`IdGenerationError::GeneratorExpired`] when the configured wall clock is
//! equal to or later than the boundary. An unrepresentable boundary returns
//! [`IdGenerationError::ExpirationTimeOverflow`] instead.
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
mod generation_attempt;
mod id;
mod id_generation_error;
mod id_generation_future;
mod id_generator;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(
        feature = "qubit-snowflake",
        feature = "classic-snowflake",
        feature = "sonyflake",
    )))
)]
pub mod snowflake;
mod try_id_generator;
#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
pub mod uuid;

pub use async_id_generator::AsyncIdGenerator;
pub use generation_attempt::GenerationAttempt;
pub use id::Id;
pub use id_generation_error::IdGenerationError;
pub use id_generation_future::IdGenerationFuture;
pub use id_generator::IdGenerator;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(
        feature = "qubit-snowflake",
        feature = "classic-snowflake",
        feature = "sonyflake",
    )))
)]
pub use snowflake::RestartPolicy;
#[cfg(feature = "classic-snowflake")]
#[cfg_attr(docsrs, doc(cfg(feature = "classic-snowflake")))]
pub use snowflake::{
    ClassicalSnowflakeGenerator,
    ClassicalSnowflakeGeneratorBuilder,
    ClassicalSnowflakeLayout,
    ClassicalSnowflakeParts,
};
#[cfg(feature = "qubit-snowflake")]
#[cfg_attr(docsrs, doc(cfg(feature = "qubit-snowflake")))]
pub use snowflake::{
    DEFAULT_MAX_CLOCK_SKEW,
    HOST_BITS,
    HOST_MAX,
    HOST_MIN,
    IdMode,
    PRECISION_BITS,
    SnowflakeGenerator,
    SnowflakeGeneratorBuilder,
    SnowflakeLayout,
    SnowflakeParts,
    TimestampPrecision,
};
#[cfg(feature = "sonyflake")]
#[cfg_attr(docsrs, doc(cfg(feature = "sonyflake")))]
pub use snowflake::{
    SonyflakeGenerator,
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
    SonyflakeParts,
};
pub use try_id_generator::TryIdGenerator;
#[cfg(feature = "uuid")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid")))]
pub use uuid::{
    UuidV4Generator,
};

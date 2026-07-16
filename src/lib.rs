// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! # Qubit ID
//!
//! ID generation utilities for Rust services. The crate provides one
//! associated-type [`IdGenerator`] contract, three Snowflake-family numeric
//! generators, and Mica-style random UUID-like values.
//!
//! ## Cargo features
//!
//! The default feature set contains only `qubit-snowflake`. The
//! `classic-snowflake`, `sonyflake`, and `uuid` algorithms are independently
//! opt-in; disabling default features without selecting another feature leaves
//! the common generator trait, outcome, and error APIs available.
//!
//! | Feature | Contents |
//! | --- | --- |
//! | `qubit-snowflake` | Qubit Snowflake layout, parts, builder, and generator |
//! | `classic-snowflake` | Classic Snowflake layout, parts, builder, and generator |
//! | `sonyflake` | Sonyflake layout, parts, builder, and generator |
//! | `uuid` | Mica UUID-like generator and string helpers |
//!
//! ## Allocation and lifetime
//!
//! [`IdGenerator::try_next_id`] performs one allocation attempt without
//! invoking a sleeper. [`IdGenerator::next_id`] may adapt a retry outcome into
//! a blocking wait. For Snowflake-family generators, configure
//! [`RestartPolicy::WaitNextSlice`] when a fresh instance should skip its first
//! observed logical time slice. This policy does not know the predecessor's
//! allocation watermark, so clock rollback across a restart can still repeat
//! IDs. The default [`RestartPolicy::Immediate`] can repeat IDs after
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
//! The UUID-like implementation can be compared locally with standard UUID v4
//! generation by running
//! `cargo bench --no-default-features --features uuid --bench uuid_comparison`.

#![deny(missing_docs)]

mod generation_outcome;
mod id_error;
mod id_generator;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
pub mod snowflake;
#[cfg(feature = "uuid")]
pub mod uuid;

pub use generation_outcome::GenerationOutcome;
pub use id_error::IdError;
pub use id_generator::IdGenerator;
#[cfg(any(
    feature = "qubit-snowflake",
    feature = "classic-snowflake",
    feature = "sonyflake",
))]
pub use snowflake::RestartPolicy;
#[cfg(feature = "qubit-snowflake")]
pub use snowflake::{
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
    SnowflakeGenerator,
    SnowflakeGeneratorBuilder,
    SnowflakeLayout,
    SnowflakeParts,
};
#[cfg(feature = "sonyflake")]
pub use snowflake::{
    SonyflakeGenerator,
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
    SonyflakeParts,
};
#[cfg(feature = "uuid")]
pub use uuid::{
    MicaUuidLikeGenerator,
    fast_simple_uuid_like,
    fast_uuid_like,
};

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
//! [`IdGenerator::try_next_id`] performs one allocation attempt without
//! invoking a sleeper. [`IdGenerator::next_id`] may adapt a retry outcome into
//! a blocking wait. For Snowflake-family generators, configure
//! [`RestartPolicy::WaitNextSlice`] when a fresh instance should skip its first
//! observed logical time slice. This policy does not know the predecessor's
//! allocation watermark, so clock rollback across a restart can still repeat
//! IDs. The default [`RestartPolicy::Immediate`] can repeat IDs after
//! same-slice allocation state is lost.
//!
//! This crate does not persist allocation state, coordinate generator
//! identities across processes, reserve a layout version field, or provide an
//! authenticity check for decoded IDs. Applications must assign exclusive
//! host, node, or machine identifiers for concurrently active generators in
//! the same namespace.

#![deny(missing_docs)]

mod generation_outcome;
mod id_error;
mod id_generator;
pub mod snowflake;
pub mod uuid;

pub use generation_outcome::GenerationOutcome;
pub use id_error::IdError;
pub use id_generator::IdGenerator;
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
    RestartPolicy,
    SnowflakeGenerator,
    SnowflakeGeneratorBuilder,
    SonyflakeGenerator,
    SonyflakeGeneratorBuilder,
    TimestampPrecision,
};
pub use uuid::{
    MicaUuidLikeGenerator,
    fast_simple_uuid_like,
    fast_uuid_like,
};

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Snowflake-family ID generators and shared implementation helpers.

#[cfg(feature = "classic-snowflake")]
pub mod classical;
mod internal;
#[cfg(feature = "qubit-snowflake")]
pub mod qubit;
mod restart_policy;
#[cfg(feature = "sonyflake")]
pub mod sonyflake;

#[cfg(feature = "classic-snowflake")]
pub use classical::{
    ClassicalSnowflakeGenerator,
    ClassicalSnowflakeGeneratorBuilder,
    ClassicalSnowflakeLayout,
    ClassicalSnowflakeParts,
};
#[cfg(feature = "qubit-snowflake")]
pub use qubit::{
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
pub use restart_policy::RestartPolicy;
#[cfg(feature = "sonyflake")]
pub use sonyflake::{
    SonyflakeGenerator,
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
    SonyflakeParts,
};

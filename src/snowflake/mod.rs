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
pub use classical::ClassicalSnowflakeGenerator;
#[cfg(feature = "classic-snowflake")]
pub use classical::ClassicalSnowflakeGeneratorBuilder;
#[cfg(feature = "classic-snowflake")]
pub use classical::ClassicalSnowflakeLayout;
#[cfg(feature = "classic-snowflake")]
pub use classical::ClassicalSnowflakeParts;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::DEFAULT_MAX_CLOCK_SKEW;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::HOST_BITS;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::HOST_MAX;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::HOST_MIN;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::IdMode;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::PRECISION_BITS;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::SnowflakeGenerator;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::SnowflakeGeneratorBuilder;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::SnowflakeLayout;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::SnowflakeParts;
#[cfg(feature = "qubit-snowflake")]
pub use qubit::TimestampPrecision;
pub use restart_policy::RestartPolicy;
#[cfg(feature = "sonyflake")]
pub use sonyflake::SonyflakeGenerator;
#[cfg(feature = "sonyflake")]
pub use sonyflake::SonyflakeGeneratorBuilder;
#[cfg(feature = "sonyflake")]
pub use sonyflake::SonyflakeLayout;
#[cfg(feature = "sonyflake")]
pub use sonyflake::SonyflakeParts;

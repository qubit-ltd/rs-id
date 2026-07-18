// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Snowflake-family ID generators and related Qubit layout helpers.

#[cfg(feature = "qubit-snowflake")]
mod async_qubit_snowflake_generator;
#[cfg(feature = "classic-snowflake")]
mod async_snowflake_generator;
#[cfg(feature = "sonyflake")]
mod async_sonyflake_generator;
#[cfg(feature = "qubit-snowflake")]
mod constants;
#[cfg(feature = "qubit-snowflake")]
mod id_mode;
mod internal;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_generator;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_generator_builder;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_layout;
#[cfg(feature = "qubit-snowflake")]
mod qubit_snowflake_parts;
mod restart_policy;
#[cfg(feature = "classic-snowflake")]
mod snowflake_generator;
#[cfg(feature = "classic-snowflake")]
mod snowflake_generator_builder;
#[cfg(feature = "classic-snowflake")]
mod snowflake_layout;
#[cfg(feature = "classic-snowflake")]
mod snowflake_parts;
mod snowflake_string_generator;
#[cfg(feature = "sonyflake")]
mod sonyflake_generator;
#[cfg(feature = "sonyflake")]
mod sonyflake_generator_builder;
#[cfg(feature = "sonyflake")]
mod sonyflake_layout;
#[cfg(feature = "sonyflake")]
mod sonyflake_parts;
#[cfg(feature = "qubit-snowflake")]
mod timestamp_precision;

#[cfg(feature = "qubit-snowflake")]
pub use async_qubit_snowflake_generator::AsyncQubitSnowflakeGenerator;
#[cfg(feature = "classic-snowflake")]
pub use async_snowflake_generator::AsyncSnowflakeGenerator;
#[cfg(feature = "sonyflake")]
pub use async_sonyflake_generator::AsyncSonyflakeGenerator;
#[cfg(feature = "qubit-snowflake")]
pub use constants::{
    DEFAULT_MAX_CLOCK_SKEW,
    HOST_BITS,
    HOST_MAX,
    HOST_MIN,
    PRECISION_BITS,
};
#[cfg(feature = "qubit-snowflake")]
pub use id_mode::IdMode;
#[cfg(feature = "qubit-snowflake")]
pub use qubit_snowflake_generator::QubitSnowflakeGenerator;
#[cfg(feature = "qubit-snowflake")]
pub use qubit_snowflake_generator_builder::QubitSnowflakeGeneratorBuilder;
#[cfg(feature = "qubit-snowflake")]
pub use qubit_snowflake_layout::QubitSnowflakeLayout;
#[cfg(feature = "qubit-snowflake")]
pub use qubit_snowflake_parts::QubitSnowflakeParts;
pub use restart_policy::RestartPolicy;
#[cfg(feature = "classic-snowflake")]
pub use snowflake_generator::SnowflakeGenerator;
#[cfg(feature = "classic-snowflake")]
pub use snowflake_generator_builder::SnowflakeGeneratorBuilder;
#[cfg(feature = "classic-snowflake")]
pub use snowflake_layout::SnowflakeLayout;
#[cfg(feature = "classic-snowflake")]
pub use snowflake_parts::SnowflakeParts;
pub use snowflake_string_generator::SnowflakeStringGenerator;
#[cfg(feature = "sonyflake")]
pub use sonyflake_generator::SonyflakeGenerator;
#[cfg(feature = "sonyflake")]
pub use sonyflake_generator_builder::SonyflakeGeneratorBuilder;
#[cfg(feature = "sonyflake")]
pub use sonyflake_layout::SonyflakeLayout;
#[cfg(feature = "sonyflake")]
pub use sonyflake_parts::SonyflakeParts;
#[cfg(feature = "qubit-snowflake")]
pub use timestamp_precision::TimestampPrecision;

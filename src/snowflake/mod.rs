// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Snowflake-family ID generators and related Qubit layout helpers.

mod constants;
mod id_mode;
mod qubit_snowflake_generator;
mod qubit_snowflake_layout;
mod qubit_snowflake_parts;
mod snowflake_generator;
mod sonyflake_generator;
mod time_slice;
mod timestamp_precision;

pub use constants::{
    DEFAULT_MAX_SKEW_MILLIS,
    HOST_BITS,
    HOST_MAX,
    HOST_MIN,
    PRECISION_BITS,
};
pub use id_mode::IdMode;
pub use qubit_snowflake_generator::QubitSnowflakeGenerator;
pub use qubit_snowflake_layout::QubitSnowflakeLayout;
pub use qubit_snowflake_parts::QubitSnowflakeParts;
pub use snowflake_generator::SnowflakeGenerator;
pub use sonyflake_generator::SonyflakeGenerator;
pub use timestamp_precision::TimestampPrecision;

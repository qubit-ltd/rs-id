// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Qubit Snowflake implementation.

mod constants;
mod id_mode;
mod snowflake_generator;
mod snowflake_generator_builder;
mod snowflake_layout;
mod snowflake_parts;
mod timestamp_precision;

pub use constants::DEFAULT_MAX_CLOCK_SKEW;
pub use constants::HOST_BITS;
pub use constants::HOST_MAX;
pub use constants::HOST_MIN;
pub use constants::PRECISION_BITS;
pub use id_mode::IdMode;
pub use snowflake_generator::SnowflakeGenerator;
pub use snowflake_generator_builder::SnowflakeGeneratorBuilder;
pub use snowflake_layout::SnowflakeLayout;
pub use snowflake_parts::SnowflakeParts;
pub use timestamp_precision::TimestampPrecision;

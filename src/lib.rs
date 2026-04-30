/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! # Qubit ID
//!
//! ID generation utilities for Rust services.
//!
//! # Author
//!
//! Haixing Hu

#![deny(missing_docs)]

mod constants;
mod fast_uuid_like_generator;
mod id_error;
mod id_generator;
mod id_mode;
mod qubit_snowflake_builder;
mod qubit_snowflake_generator;
mod snowflake_generator;
mod sonyflake_generator;
mod time_slice;
mod timestamp_precision;

pub use constants::{DEFAULT_MAX_SKEW_MILLIS, HOST_BITS, HOST_MAX, HOST_MIN, PRECISION_BITS};
pub use fast_uuid_like_generator::{FastUuidLikeGenerator, fast_simple_uuid_like, fast_uuid_like};
pub use id_error::IdError;
pub use id_generator::IdGenerator;
pub use id_mode::IdMode;
pub use qubit_snowflake_builder::QubitSnowflakeBuilder;
pub use qubit_snowflake_generator::QubitSnowflakeGenerator;
pub use snowflake_generator::SnowflakeGenerator;
pub use sonyflake_generator::SonyflakeGenerator;
pub use timestamp_precision::TimestampPrecision;

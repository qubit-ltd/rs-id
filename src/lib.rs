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

mod id_error;
mod id_generator;
pub mod snowflake;
pub mod uuid;

pub use id_error::IdError;
pub use id_generator::IdGenerator;
pub use snowflake::{
    DEFAULT_MAX_SKEW_MILLIS, HOST_BITS, HOST_MAX, HOST_MIN, IdMode, PRECISION_BITS,
    QubitSnowflakeBuilder, QubitSnowflakeGenerator, SnowflakeGenerator, SonyflakeGenerator,
    TimestampPrecision,
};
pub use uuid::{MicaUuidLikeGenerator, fast_simple_uuid_like, fast_uuid_like};

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Error type returned by ID generators.

use std::time::Duration;
use std::time::SystemTime;

#[cfg(feature = "uuid")]
use getrandom::Error as RandomSourceError;
#[cfg(any(feature = "qubit-snowflake", feature = "classic-snowflake", feature = "sonyflake",))]
use qubit_clock::TimeError;
use thiserror::Error;

/// Error returned when an ID generator cannot create or compose an ID.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IdGenerationError {
    /// A Qubit snowflake host identifier is outside its bit range.
    #[error("host id {host} is out of range 0..={max}")]
    HostOutOfRange {
        /// Provided host identifier.
        host: u64,
        /// Maximum valid host identifier.
        max: u64,
    },
    /// A classic snowflake node identifier is outside its bit range.
    #[error("node id {node_id} is out of range 0..={max}")]
    NodeOutOfRange {
        /// Provided node identifier.
        node_id: u64,
        /// Maximum valid node identifier.
        max: u64,
    },
    /// A Sonyflake machine identifier is outside its bit range.
    #[error("machine id {machine_id} is out of range 0..={max}")]
    MachineIdOutOfRange {
        /// Provided machine identifier.
        machine_id: u64,
        /// Maximum valid machine identifier.
        max: u64,
    },
    /// A timestamp or elapsed time is too large for the configured bit layout.
    #[error("timestamp {timestamp} exceeds maximum {max}")]
    TimestampOverflow {
        /// Provided timestamp or elapsed time.
        timestamp: u64,
        /// Maximum representable timestamp or elapsed time.
        max: u64,
    },
    /// A sequence number is too large for the configured bit layout.
    #[error("sequence {sequence} exceeds maximum {max}")]
    SequenceOverflow {
        /// Provided sequence number.
        sequence: u64,
        /// Maximum representable sequence number.
        max: u64,
    },
    /// The observed clock moved backwards beyond the configured tolerance.
    #[error(
        "clock moved backwards from {last_elapsed:?} to {current_elapsed:?}; \
         skew {skew:?} exceeds maximum {max_skew:?}"
    )]
    ClockMovedBackwards {
        /// Greatest elapsed time observed by the generator.
        last_elapsed: Duration,
        /// Elapsed time reported by the current wall-clock observation.
        current_elapsed: Duration,
        /// Difference between the last and current elapsed times.
        skew: Duration,
        /// Maximum tolerated backwards movement.
        max_skew: Duration,
    },
    /// The requested time is before the configured epoch.
    #[error("time {time:?} is before the configured epoch {epoch:?}")]
    TimeBeforeEpoch {
        /// Wall time that could not be represented relative to the epoch.
        time: SystemTime,
        /// Configured timestamp epoch.
        epoch: SystemTime,
    },
    /// The configured epoch is ahead of the generator clock.
    #[error("epoch {epoch:?} is ahead of generator clock {current_time:?}")]
    EpochAhead {
        /// Configured epoch.
        epoch: SystemTime,
        /// Wall time observed while validating the builder.
        current_time: SystemTime,
    },
    /// A Sonyflake bit length setting is invalid.
    #[error("invalid bit length for {name}: {bits}; {reason}")]
    InvalidBitLength {
        /// Name of the invalid bit field.
        name: &'static str,
        /// Provided bit length.
        bits: u8,
        /// Human-readable constraint for the field.
        reason: &'static str,
    },
    /// A Sonyflake time unit is invalid.
    #[error("invalid time unit {nanos} ns; minimum is {min_nanos} ns")]
    InvalidTimeUnit {
        /// Provided time unit in nanoseconds.
        nanos: u128,
        /// Minimum allowed time unit in nanoseconds.
        min_nanos: u128,
    },
    /// The exclusive expiration cannot be represented by [`SystemTime`].
    #[error(
        "expiration time overflows SystemTime for origin {origin:?}, time unit \
         {time_unit:?}, and maximum timestamp {max_timestamp}"
    )]
    ExpirationTimeOverflow {
        /// Configured timestamp origin.
        origin: SystemTime,
        /// Duration represented by one encoded timestamp unit.
        time_unit: Duration,
        /// Maximum encoded timestamp supported by the layout.
        max_timestamp: u64,
    },
    /// The generator's exclusive lifetime boundary has been reached.
    #[error(
        "generator expired at {expires_at:?}; observed wall time was \
         {observed_at:?}"
    )]
    GeneratorExpired {
        /// Wall time observed by the generation attempt.
        observed_at: SystemTime,
        /// Exclusive expiration boundary cached by the generator.
        expires_at: SystemTime,
    },
    /// The operating system could not provide random bytes for UUID v4.
    #[cfg(feature = "uuid")]
    #[error("failed to obtain random bytes for UUID v4")]
    RandomSourceFailed {
        /// Error returned by the operating-system random source.
        #[source]
        source: RandomSourceError,
    },
    /// The injected timer could not register or complete a retry wait.
    #[cfg(any(feature = "qubit-snowflake", feature = "classic-snowflake", feature = "sonyflake",))]
    #[error("failed to wait before retrying ID generation")]
    WaitFailed {
        /// Error returned by the injected timer or its blocking adapter.
        #[source]
        source: TimeError,
    },
}

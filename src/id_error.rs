// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Error type returned by ID generators.

use std::time::{
    Duration,
    SystemTime,
};

use thiserror::Error;

/// Error returned when an ID generator cannot create or compose an ID.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum IdError {
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
        /// Configured epoch or Sonyflake start time.
        epoch: SystemTime,
    },
    /// The configured Sonyflake start time is ahead of the generator clock.
    #[error(
        "start time {start_time:?} is ahead of generator clock {current_time:?}"
    )]
    StartTimeAhead {
        /// Configured Sonyflake start time.
        start_time: SystemTime,
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
    /// The operating system random source could not provide random ID bytes.
    #[error("operating system random source is unavailable")]
    RandomSourceUnavailable {
        /// Error returned by `getrandom`.
        #[source]
        source: getrandom::Error,
    },
    /// The injected blocking sleeper could not complete a retry wait.
    #[error("failed to wait before retrying ID generation")]
    SleepFailed {
        /// Error returned by the injected blocking sleeper.
        #[source]
        source: qubit_clock::TimeError,
    },
}

/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Error type returned by ID generators.

use std::error::Error;
use std::fmt::{
    self,
    Display,
    Formatter,
};

/// Error returned when an ID generator cannot create or compose an ID.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum IdError {
    /// A Qubit snowflake host identifier is outside its bit range.
    HostOutOfRange {
        /// Provided host identifier.
        host: u64,
        /// Maximum valid host identifier.
        max: u64,
    },
    /// A classic snowflake node identifier is outside its bit range.
    NodeOutOfRange {
        /// Provided node identifier.
        node_id: u64,
        /// Maximum valid node identifier.
        max: u64,
    },
    /// A Sonyflake machine identifier is outside its bit range.
    MachineIdOutOfRange {
        /// Provided machine identifier.
        machine_id: u64,
        /// Maximum valid machine identifier.
        max: u64,
    },
    /// A timestamp or elapsed time is too large for the configured bit layout.
    TimestampOverflow {
        /// Provided timestamp or elapsed time.
        timestamp: u64,
        /// Maximum representable timestamp or elapsed time.
        max: u64,
    },
    /// A sequence number is too large for the configured bit layout.
    SequenceOverflow {
        /// Provided sequence number.
        sequence: u64,
        /// Maximum representable sequence number.
        max: u64,
    },
    /// The observed clock moved backwards beyond the configured tolerance.
    ClockMovedBackwards {
        /// Last timestamp seen by the generator.
        last_timestamp: u64,
        /// Current timestamp reported by the clock.
        current_timestamp: u64,
        /// Backwards skew in milliseconds.
        skew_millis: u64,
        /// Maximum tolerated backwards skew in milliseconds.
        max_skew_millis: u64,
    },
    /// The requested time is before the configured epoch.
    TimeBeforeEpoch,
    /// The configured Sonyflake start time is ahead of the generator clock.
    StartTimeAhead,
    /// A Sonyflake bit length setting is invalid.
    InvalidBitLength {
        /// Name of the invalid bit field.
        name: &'static str,
        /// Provided bit length.
        bits: u8,
        /// Human-readable constraint for the field.
        reason: &'static str,
    },
    /// A Sonyflake time unit is invalid.
    InvalidTimeUnit {
        /// Provided time unit in nanoseconds.
        nanos: u128,
        /// Minimum allowed time unit in nanoseconds.
        min_nanos: u128,
    },
    /// The operating system random source could not provide random ID bytes.
    RandomSourceUnavailable,
    /// The generator state mutex was poisoned by a panic while locked.
    StatePoisoned,
}

impl Display for IdError {
    /// Formats the error with enough context for diagnostics.
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::HostOutOfRange { host, max } => {
                write!(formatter, "host id {host} is out of range 0..={max}")
            }
            Self::NodeOutOfRange { node_id, max } => {
                write!(formatter, "node id {node_id} is out of range 0..={max}")
            }
            Self::MachineIdOutOfRange { machine_id, max } => {
                write!(formatter, "machine id {machine_id} is out of range 0..={max}")
            }
            Self::TimestampOverflow { timestamp, max } => {
                write!(formatter, "timestamp {timestamp} exceeds maximum {max}")
            }
            Self::SequenceOverflow { sequence, max } => {
                write!(formatter, "sequence {sequence} exceeds maximum {max}")
            }
            Self::ClockMovedBackwards {
                last_timestamp,
                current_timestamp,
                skew_millis,
                max_skew_millis,
            } => write!(
                formatter,
                "clock moved backwards from {last_timestamp} to {current_timestamp}; \
                 skew {skew_millis} ms exceeds maximum {max_skew_millis} ms"
            ),
            Self::TimeBeforeEpoch => {
                write!(formatter, "time is before the configured epoch")
            }
            Self::StartTimeAhead => {
                write!(formatter, "start time is ahead of the generator clock")
            }
            Self::InvalidBitLength { name, bits, reason } => {
                write!(formatter, "invalid bit length for {name}: {bits}; {reason}")
            }
            Self::InvalidTimeUnit { nanos, min_nanos } => {
                write!(formatter, "invalid time unit {nanos} ns; minimum is {min_nanos} ns")
            }
            Self::RandomSourceUnavailable => {
                write!(formatter, "operating system random source is unavailable")
            }
            Self::StatePoisoned => {
                write!(formatter, "generator state mutex is poisoned")
            }
        }
    }
}

impl Error for IdError {}

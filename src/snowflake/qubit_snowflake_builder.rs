/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Qubit snowflake ID bit builder.

use super::constants::{HOST_BITS, HOST_MAX, MODE_BITS, PRECISION_BITS};
use super::{IdMode, TimestampPrecision};
use crate::IdError;

/// Builds and extracts Qubit snowflake IDs.
///
/// The layout is:
///
/// ```text
/// [mode:1][precision:1][timestamp][host:9][sequence]
/// ```
///
/// The fixed high-bit header keeps mode and precision readable without knowing
/// the timestamp and sequence widths first.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QubitSnowflakeBuilder {
    mode: IdMode,
    precision: TimestampPrecision,
    host: u64,
    mode_shift: u8,
    timestamp_shift: u8,
    precision_shift: u8,
    host_shift: u8,
    timestamp_bits: u8,
    max_timestamp: u64,
    max_sequence: u64,
    fixed_data: u64,
}

impl QubitSnowflakeBuilder {
    /// Creates a Qubit snowflake builder.
    ///
    /// # Parameters
    /// - `mode`: Encoded ID ordering mode.
    /// - `precision`: Encoded timestamp precision.
    /// - `host`: Host identifier in `0..=511`.
    ///
    /// # Returns
    /// A configured builder.
    ///
    /// # Errors
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit in the 9-bit
    /// host field.
    pub fn new(mode: IdMode, precision: TimestampPrecision, host: u64) -> Result<Self, IdError> {
        if host > HOST_MAX {
            return Err(IdError::HostOutOfRange {
                host,
                max: HOST_MAX,
            });
        }
        Ok(Self::new_unchecked(mode, precision, host))
    }

    /// Creates a builder after the caller has validated the host field.
    ///
    /// # Parameters
    /// - `mode`: Encoded ID ordering mode.
    /// - `precision`: Encoded timestamp precision.
    /// - `host`: Valid host identifier.
    ///
    /// # Returns
    /// A configured builder.
    fn new_unchecked(mode: IdMode, precision: TimestampPrecision, host: u64) -> Self {
        let timestamp_bits = precision.timestamp_bits();
        let sequence_bits = precision.sequence_bits();
        let max_timestamp = (1_u64 << timestamp_bits) - 1;
        let max_sequence = (1_u64 << sequence_bits) - 1;
        let mode_shift = u64::BITS as u8 - MODE_BITS;
        let precision_shift = mode_shift - PRECISION_BITS;
        let timestamp_shift = HOST_BITS + sequence_bits;
        let host_shift = sequence_bits;
        let fixed_data = (mode.ordinal() << mode_shift)
            | (precision.ordinal() << precision_shift)
            | (host << host_shift);

        Self {
            mode,
            precision,
            host,
            mode_shift,
            timestamp_shift,
            precision_shift,
            host_shift,
            timestamp_bits,
            max_timestamp,
            max_sequence,
            fixed_data,
        }
    }

    /// Returns the encoded mode.
    ///
    /// # Returns
    /// ID ordering mode.
    pub const fn mode(&self) -> IdMode {
        self.mode
    }

    /// Returns the encoded timestamp precision.
    ///
    /// # Returns
    /// Timestamp precision.
    pub const fn precision(&self) -> TimestampPrecision {
        self.precision
    }

    /// Returns the encoded host identifier.
    ///
    /// # Returns
    /// Host identifier.
    pub const fn host(&self) -> u64 {
        self.host
    }

    /// Returns the maximum representable timestamp.
    ///
    /// # Returns
    /// Maximum timestamp for the configured precision.
    pub const fn max_timestamp(&self) -> u64 {
        self.max_timestamp
    }

    /// Returns the maximum representable sequence number.
    ///
    /// # Returns
    /// Maximum sequence for the configured precision.
    pub const fn max_sequence(&self) -> u64 {
        self.max_sequence
    }

    /// Builds an ID from timestamp and sequence parts.
    ///
    /// # Parameters
    /// - `timestamp`: Timestamp measured from the configured epoch in the
    ///   configured precision.
    /// - `sequence`: Sequence value inside the timestamp slice.
    ///
    /// # Returns
    /// Encoded ID.
    ///
    /// # Errors
    /// Returns [`IdError::TimestampOverflow`] when `timestamp` exceeds the
    /// configured timestamp field. Returns [`IdError::SequenceOverflow`] when
    /// `sequence` exceeds the configured sequence field.
    pub fn build(&self, timestamp: u64, sequence: u64) -> Result<u64, IdError> {
        if timestamp > self.max_timestamp {
            return Err(IdError::TimestampOverflow {
                timestamp,
                max: self.max_timestamp,
            });
        }
        if sequence > self.max_sequence {
            return Err(IdError::SequenceOverflow {
                sequence,
                max: self.max_sequence,
            });
        }
        let stored_timestamp = match self.mode {
            IdMode::Sequential => timestamp,
            IdMode::Spread => timestamp.reverse_bits() >> (u64::BITS as u8 - self.timestamp_bits),
        };
        Ok((stored_timestamp << self.timestamp_shift) | self.fixed_data | sequence)
    }

    /// Extracts the encoded ID ordering mode.
    ///
    /// # Parameters
    /// - `id`: ID generated with the Qubit layout.
    ///
    /// # Returns
    /// Encoded ordering mode.
    pub fn extract_mode(&self, id: u64) -> IdMode {
        let bit = (id >> self.mode_shift) & ((1_u64 << MODE_BITS) - 1);
        IdMode::from_bit(bit)
    }

    /// Extracts the encoded timestamp.
    ///
    /// # Parameters
    /// - `id`: ID generated with this builder.
    ///
    /// # Returns
    /// Original timestamp before optional spread-mode bit reversal.
    pub fn extract_timestamp(&self, id: u64) -> u64 {
        let timestamp = (id >> self.timestamp_shift) & self.max_timestamp;
        match self.mode {
            IdMode::Sequential => timestamp,
            IdMode::Spread => timestamp.reverse_bits() >> (u64::BITS as u8 - self.timestamp_bits),
        }
    }

    /// Extracts the encoded timestamp precision.
    ///
    /// # Parameters
    /// - `id`: ID generated with the Qubit layout.
    ///
    /// # Returns
    /// Encoded timestamp precision.
    pub fn extract_precision(&self, id: u64) -> TimestampPrecision {
        let bit = (id >> self.precision_shift) & ((1_u64 << PRECISION_BITS) - 1);
        TimestampPrecision::from_bit(bit)
    }

    /// Extracts the encoded host identifier.
    ///
    /// # Parameters
    /// - `id`: ID generated with the Qubit layout.
    ///
    /// # Returns
    /// Host identifier.
    pub fn extract_host(&self, id: u64) -> u64 {
        (id >> self.host_shift) & ((1_u64 << HOST_BITS) - 1)
    }

    /// Extracts the encoded sequence.
    ///
    /// # Parameters
    /// - `id`: ID generated with this builder.
    ///
    /// # Returns
    /// Sequence number.
    pub fn extract_sequence(&self, id: u64) -> u64 {
        id & self.max_sequence
    }
}

impl Default for QubitSnowflakeBuilder {
    /// Creates the default Qubit layout.
    fn default() -> Self {
        Self::new_unchecked(IdMode::Sequential, TimestampPrecision::Second, 0)
    }
}

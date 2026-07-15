// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Qubit snowflake ID bit layout.

use super::constants::{
    HOST_BITS,
    HOST_MAX,
    MODE_BITS,
    PRECISION_BITS,
};
use super::{
    IdMode,
    QubitSnowflakeParts,
    TimestampPrecision,
};
use crate::IdError;

/// Immutable Qubit snowflake bit layout used to compose IDs.
///
/// The layout is:
///
/// ```text
/// [mode:1][precision:1][timestamp][host:9][sequence]
/// ```
///
/// [`Self::compose`] and [`Self::decode`] are stateless bit operations. They do
/// not allocate an ID and provide no uniqueness guarantee.
///
/// [`IdMode::Spread`] reversibly obscures the numeric relationship between
/// adjacent timestamp slices. It is intended to make simple ordering and
/// volume inference from public IDs harder, not to provide encryption.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct QubitSnowflakeLayout {
    mode: IdMode,
    precision: TimestampPrecision,
    host: u64,
    timestamp_shift: u8,
    host_shift: u8,
    timestamp_bits: u8,
    max_timestamp: u64,
    max_sequence: u64,
    fixed_data: u64,
}

impl QubitSnowflakeLayout {
    /// Creates a Qubit snowflake layout.
    ///
    /// # Parameters
    /// - `mode`: Encoded ID ordering mode.
    /// - `precision`: Encoded timestamp precision.
    /// - `host`: Host identifier in `0..=511`.
    ///
    /// # Returns
    /// A configured layout.
    ///
    /// # Errors
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit in the
    /// 9-bit host field.
    pub fn new(
        mode: IdMode,
        precision: TimestampPrecision,
        host: u64,
    ) -> Result<Self, IdError> {
        if host > HOST_MAX {
            return Err(IdError::HostOutOfRange {
                host,
                max: HOST_MAX,
            });
        }
        Ok(Self::new_unchecked(mode, precision, host))
    }

    /// Creates a layout after the caller has validated the host field.
    ///
    /// # Parameters
    /// - `mode`: Encoded ID ordering mode.
    /// - `precision`: Encoded timestamp precision.
    /// - `host`: Valid host identifier.
    ///
    /// # Returns
    /// A configured layout.
    fn new_unchecked(
        mode: IdMode,
        precision: TimestampPrecision,
        host: u64,
    ) -> Self {
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
            timestamp_shift,
            host_shift,
            timestamp_bits,
            max_timestamp,
            max_sequence,
            fixed_data,
        }
    }

    /// Returns the encoded mode.
    #[inline(always)]
    pub const fn mode(&self) -> IdMode {
        self.mode
    }

    /// Returns the encoded timestamp precision.
    #[inline(always)]
    pub const fn precision(&self) -> TimestampPrecision {
        self.precision
    }

    /// Returns the encoded host identifier.
    #[inline(always)]
    pub const fn host(&self) -> u64 {
        self.host
    }

    /// Returns the maximum representable timestamp.
    #[inline(always)]
    pub const fn max_timestamp(&self) -> u64 {
        self.max_timestamp
    }

    /// Returns the maximum representable sequence number.
    #[inline(always)]
    pub const fn max_sequence(&self) -> u64 {
        self.max_sequence
    }

    /// Composes an ID from timestamp and sequence parts.
    ///
    /// This method is stateless and does not guarantee uniqueness.
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
    /// Returns [`IdError::TimestampOverflow`] or
    /// [`IdError::SequenceOverflow`] when a part does not fit.
    pub fn compose(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<u64, IdError> {
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
        let stored_timestamp = Self::transform_timestamp(
            self.mode,
            self.timestamp_bits,
            timestamp,
        );
        Ok((stored_timestamp << self.timestamp_shift)
            | self.fixed_data
            | sequence)
    }

    /// Decodes a Qubit snowflake ID without a preconfigured layout.
    ///
    /// Mode and precision are read from the fixed high-bit header before the
    /// remaining field widths are derived. Every `u64` value is a structurally
    /// valid Qubit bit pattern, so decoding is infallible.
    ///
    /// # Parameters
    /// - `id`: Qubit snowflake bit pattern to decode.
    ///
    /// # Returns
    /// All fields decoded using the layout encoded in `id`.
    pub fn decode(id: u64) -> QubitSnowflakeParts {
        let mode_shift = u64::BITS as u8 - MODE_BITS;
        let precision_shift = mode_shift - PRECISION_BITS;
        let mode = IdMode::from_bit((id >> mode_shift) & 1);
        let precision =
            TimestampPrecision::from_bit((id >> precision_shift) & 1);
        let layout = Self::new_unchecked(mode, precision, 0);
        let stored_timestamp =
            (id >> layout.timestamp_shift) & layout.max_timestamp;
        let timestamp = Self::transform_timestamp(
            mode,
            layout.timestamp_bits,
            stored_timestamp,
        );
        let host = (id >> layout.host_shift) & ((1_u64 << HOST_BITS) - 1);
        let sequence = id & layout.max_sequence;
        QubitSnowflakeParts::new(mode, precision, timestamp, host, sequence)
    }

    /// Applies the reversible timestamp transform for the configured mode.
    ///
    /// # Parameters
    /// - `mode`: Timestamp storage mode.
    /// - `timestamp_bits`: Width of the timestamp field.
    /// - `timestamp`: Timestamp to transform.
    ///
    /// # Returns
    /// Transformed timestamp restricted to `timestamp_bits` significant bits.
    fn transform_timestamp(
        mode: IdMode,
        timestamp_bits: u8,
        timestamp: u64,
    ) -> u64 {
        match mode {
            IdMode::Sequential => timestamp,
            IdMode::Spread => {
                timestamp.reverse_bits() >> (u64::BITS as u8 - timestamp_bits)
            }
        }
    }
}

impl Default for QubitSnowflakeLayout {
    /// Creates the default Qubit layout.
    #[inline(always)]
    fn default() -> Self {
        Self::new_unchecked(IdMode::Sequential, TimestampPrecision::Second, 0)
    }
}

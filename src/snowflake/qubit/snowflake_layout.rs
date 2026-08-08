// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Qubit snowflake ID bit layout.

use std::time::Duration;
use std::time::SystemTime;

use super::super::internal::SnowflakeLayoutSpec;
use super::super::internal::expiration_time;
use super::IdMode;
use super::SnowflakeParts;
use super::TimestampPrecision;
use super::constants::HOST_BITS;
use super::constants::HOST_MAX;
use super::constants::MODE_BITS;
use super::constants::PRECISION_BITS;
use crate::Id;
use crate::IdGenerationError;

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
/// Spread IDs always set bit 63 and therefore always exceed `i64::MAX`. Store
/// them as unsigned 64-bit values, decimal strings, or binary data; use
/// strings when crossing JavaScript-style safe-integer boundaries.
///
/// The 64-bit layout reserves neither a sign bit nor a version field. This is
/// an intentional capacity and throughput trade-off. A future incompatible
/// layout must use a new explicit type or API rather than silently changing
/// this one.
///
/// Decoding an arbitrary `u64` only extracts fields according to the layout.
/// It does not prove that the value was produced by this generator and is not
/// an authenticity or format-validation operation.
#[derive(Debug, Clone, Eq, PartialEq)]
#[must_use]
pub struct SnowflakeLayout {
    /// ID ordering mode encoded in the high-bit header.
    mode: IdMode,
    /// Timestamp precision encoded in the high-bit header.
    precision: TimestampPrecision,
    /// Host identifier encoded in composed IDs.
    host: u64,
    /// Number of low bits below the timestamp field.
    timestamp_shift: u8,
    /// Number of low bits below the host field.
    host_shift: u8,
    /// Width of the timestamp field.
    timestamp_bits: u8,
    /// Maximum timestamp representable by the layout.
    max_timestamp: u64,
    /// Maximum sequence representable by the layout.
    max_sequence: u64,
    /// Precomputed mode, precision, and host bit fields.
    fixed_data: u64,
}

impl SnowflakeLayout {
    /// Creates a Qubit snowflake layout.
    ///
    /// # Parameters
    ///
    /// * `mode` - Encoded ID ordering mode.
    /// * `precision` - Encoded timestamp precision.
    /// * `host` - Host identifier in `0..=511`.
    ///
    /// # Returns
    ///
    /// A configured layout.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::HostOutOfRange`] when `host` does not fit
    /// in the 9-bit host field.
    #[inline]
    pub fn new(
        mode: IdMode,
        precision: TimestampPrecision,
        host: u64,
    ) -> Result<Self, IdGenerationError> {
        if host > HOST_MAX {
            return Err(IdGenerationError::HostOutOfRange {
                host,
                max: HOST_MAX,
            });
        }
        Ok(Self::new_unchecked(mode, precision, host))
    }

    /// Creates a layout after the caller has validated the host field.
    ///
    /// # Parameters
    ///
    /// * `mode` - Encoded ID ordering mode.
    /// * `precision` - Encoded timestamp precision.
    /// * `host` - Valid host identifier.
    ///
    /// # Returns
    ///
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
    ///
    /// # Returns
    ///
    /// ID ordering mode encoded by this layout.
    #[inline(always)]
    pub const fn mode(&self) -> IdMode {
        self.mode
    }

    /// Returns the encoded timestamp precision.
    ///
    /// # Returns
    ///
    /// Timestamp precision encoded by this layout.
    #[inline(always)]
    pub const fn precision(&self) -> TimestampPrecision {
        self.precision
    }

    /// Returns the encoded host identifier.
    ///
    /// # Returns
    ///
    /// Host identifier encoded by this layout.
    #[must_use]
    #[inline(always)]
    pub const fn host(&self) -> u64 {
        self.host
    }

    /// Returns the maximum representable timestamp.
    ///
    /// # Returns
    ///
    /// Maximum timestamp accepted by [`Self::compose`].
    #[must_use]
    #[inline(always)]
    pub const fn max_timestamp(&self) -> u64 {
        self.max_timestamp
    }

    /// Returns the maximum representable sequence number.
    ///
    /// # Returns
    ///
    /// Maximum sequence accepted by [`Self::compose`].
    #[must_use]
    #[inline(always)]
    pub const fn max_sequence(&self) -> u64 {
        self.max_sequence
    }

    /// Calculates this layout's exclusive expiration for an epoch.
    ///
    /// Timestamp values from zero through [`Self::max_timestamp`] are valid.
    /// The returned time is the first instant outside that range.
    ///
    /// # Parameters
    ///
    /// * `epoch` - Timestamp origin represented by timestamp zero.
    ///
    /// # Returns
    ///
    /// The exclusive expiration boundary.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::ExpirationTimeOverflow`] when the boundary
    /// cannot be represented by [`SystemTime`].
    #[inline(always)]
    pub fn expires_at(
        &self,
        epoch: SystemTime,
    ) -> Result<SystemTime, IdGenerationError> {
        expiration_time(
            epoch,
            Duration::from_millis(self.precision.divisor_millis()),
            self.max_timestamp,
        )
    }

    /// Composes an ID from timestamp and sequence parts.
    ///
    /// This method is stateless and does not guarantee uniqueness.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Timestamp measured from the configured epoch in the
    ///   configured precision.
    /// * `sequence` - Sequence value inside the timestamp slice.
    ///
    /// # Returns
    ///
    /// Encoded ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] or
    /// [`IdGenerationError::SequenceOverflow`] when a part does not fit.
    pub fn compose_raw(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<u64, IdGenerationError> {
        if timestamp > self.max_timestamp {
            return Err(IdGenerationError::TimestampOverflow {
                timestamp,
                max: self.max_timestamp,
            });
        }
        if sequence > self.max_sequence {
            return Err(IdGenerationError::SequenceOverflow {
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
    /// decodable Qubit bit pattern, so decoding is infallible. Decoding only
    /// extracts fields; it does not authenticate the value or prove that a
    /// generator produced it.
    ///
    /// # Parameters
    ///
    /// * `id` - Qubit snowflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// All fields decoded using the layout encoded in `id`.
    pub fn decode(id: Id) -> SnowflakeParts {
        Self::decode_raw(id.value())
    }

    /// Decodes a Qubit snowflake bit pattern without a preconfigured layout.
    ///
    /// # Parameters
    ///
    /// * `id` - Qubit snowflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// All fields decoded using the layout encoded in `id`.
    pub fn decode_raw(id: u64) -> SnowflakeParts {
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
        SnowflakeParts::new(mode, precision, timestamp, host, sequence)
    }

    /// Composes an ID from timestamp and sequence parts.
    ///
    /// # Errors
    ///
    /// Returns the same overflow errors as [`Self::compose_raw`].
    #[inline(always)]
    pub fn compose(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<Id, IdGenerationError> {
        self.compose_raw(timestamp, sequence).map(Id::from)
    }

    /// Applies the reversible timestamp transform for the configured mode.
    ///
    /// # Parameters
    ///
    /// * `mode` - Timestamp storage mode.
    /// * `timestamp_bits` - Width of the timestamp field.
    /// * `timestamp` - Timestamp to transform.
    ///
    /// # Returns
    ///
    /// Transformed timestamp restricted to `timestamp_bits` significant bits.
    #[must_use]
    #[inline]
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

impl Default for SnowflakeLayout {
    /// Creates the default Qubit layout.
    ///
    /// # Returns
    ///
    /// Sequential, second-precision layout for host zero.
    #[inline(always)]
    fn default() -> Self {
        Self::new_unchecked(IdMode::Sequential, TimestampPrecision::Second, 0)
    }
}

impl SnowflakeLayoutSpec for SnowflakeLayout {
    /// Returns the duration represented by one Qubit timestamp unit.
    ///
    /// # Returns
    ///
    /// The duration selected by this layout's timestamp precision.
    #[inline(always)]
    fn time_unit(&self) -> Duration {
        Duration::from_millis(self.precision.divisor_millis())
    }

    /// Returns the greatest Qubit timestamp accepted by this layout.
    ///
    /// # Returns
    ///
    /// The maximum encoded timestamp.
    #[inline(always)]
    fn max_timestamp(&self) -> u64 {
        SnowflakeLayout::max_timestamp(self)
    }

    /// Returns the greatest Qubit sequence accepted by this layout.
    ///
    /// # Returns
    ///
    /// The maximum sequence within one timestamp unit.
    #[inline(always)]
    fn max_sequence(&self) -> u64 {
        SnowflakeLayout::max_sequence(self)
    }

    /// Composes a Qubit ID from a timestamp and sequence.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Encoded timestamp relative to the configured epoch.
    /// * `sequence` - Sequence allocated within the timestamp unit.
    ///
    /// # Returns
    ///
    /// The composed Qubit Snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] or
    /// [`IdGenerationError::SequenceOverflow`] when a value exceeds its field.
    #[inline(always)]
    fn compose(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<u64, IdGenerationError> {
        SnowflakeLayout::compose_raw(self, timestamp, sequence)
    }
}

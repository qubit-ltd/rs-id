// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stateless configurable Sonyflake bit layout.

use std::time::Duration;
use std::time::SystemTime;

use super::super::internal::SnowflakeLayoutSpec;
use super::super::internal::expiration_time;
use super::sonyflake_parts::SonyflakeParts;
use crate::Id;
use crate::IdGenerationError;

/// Default number of bits used for the sequence field.
pub(super) const DEFAULT_BITS_SEQUENCE: u8 = 8;
/// Default number of bits used for the machine field.
pub(super) const DEFAULT_BITS_MACHINE: u8 = 16;
/// Default duration of one elapsed-time unit in nanoseconds.
pub(super) const DEFAULT_TIME_UNIT_NANOS: u128 = 10_000_000;
/// Minimum supported elapsed-time unit in nanoseconds.
const MIN_TIME_UNIT_NANOS: u128 = 1_000_000;

/// Immutable configurable Sonyflake bit layout.
///
/// The layout owns the machine identifier used by [`Self::compose`].
/// Composing and decoding are stateless bit operations and provide no
/// uniqueness guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SonyflakeLayout {
    /// Width of the elapsed-time field.
    bits_time: u8,
    /// Width of the sequence field.
    bits_sequence: u8,
    /// Width of the machine field.
    bits_machine: u8,
    /// Duration represented by one elapsed-time unit.
    time_unit: Duration,
    /// Machine identifier encoded by composed IDs.
    machine_id: u64,
}

impl SonyflakeLayout {
    /// Creates a configurable Sonyflake layout.
    ///
    /// Zero sequence or machine widths select the respective defaults of 8
    /// and 16 bits. The remaining bits out of 63 are assigned to elapsed time.
    ///
    /// # Parameters
    ///
    /// * `machine_id` - Machine identifier encoded by composed IDs.
    /// * `bits_sequence` - Sequence field width, or zero for the default.
    /// * `bits_machine` - Machine field width, or zero for the default.
    /// * `time_unit` - Duration represented by one elapsed-time unit.
    ///
    /// # Returns
    ///
    /// A validated Sonyflake layout.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::InvalidBitLength`] when the requested
    /// fields cannot leave at least 32 time bits,
    /// [`IdGenerationError::InvalidTimeUnit`] when `time_unit` is shorter
    /// than one millisecond, or [`IdGenerationError::MachineIdOutOfRange`]
    /// when `machine_id` does not fit the selected machine field.
    pub fn new(
        machine_id: u64,
        bits_sequence: u8,
        bits_machine: u8,
        time_unit: Duration,
    ) -> Result<Self, IdGenerationError> {
        let bits_sequence = Self::normalize_bits(
            "sequence",
            bits_sequence,
            DEFAULT_BITS_SEQUENCE,
        )?;
        let bits_machine = Self::normalize_bits(
            "machine",
            bits_machine,
            DEFAULT_BITS_MACHINE,
        )?;
        let bits_time = 63_u8 - bits_sequence - bits_machine;
        if bits_time < 32 {
            return Err(IdGenerationError::InvalidBitLength {
                name: "time",
                bits: bits_time,
                reason: "time bit length must be at least 32",
            });
        }

        let nanos = time_unit.as_nanos();
        if nanos < MIN_TIME_UNIT_NANOS {
            return Err(IdGenerationError::InvalidTimeUnit {
                nanos,
                min_nanos: MIN_TIME_UNIT_NANOS,
            });
        }

        let max_machine_id = (1_u64 << bits_machine) - 1;
        if machine_id > max_machine_id {
            return Err(IdGenerationError::MachineIdOutOfRange {
                machine_id,
                max: max_machine_id,
            });
        }

        Ok(Self {
            bits_time,
            bits_sequence,
            bits_machine,
            time_unit,
            machine_id,
        })
    }

    /// Returns the number of elapsed-time bits.
    ///
    /// # Returns
    ///
    /// Elapsed-time field width.
    #[must_use]
    #[inline(always)]
    pub const fn bits_time(&self) -> u8 {
        self.bits_time
    }

    /// Returns the number of sequence bits.
    ///
    /// # Returns
    ///
    /// Sequence field width.
    #[must_use]
    #[inline(always)]
    pub const fn bits_sequence(&self) -> u8 {
        self.bits_sequence
    }

    /// Returns the number of machine bits.
    ///
    /// # Returns
    ///
    /// Machine field width.
    #[must_use]
    #[inline(always)]
    pub const fn bits_machine(&self) -> u8 {
        self.bits_machine
    }

    /// Returns the configured time unit.
    ///
    /// # Returns
    ///
    /// Duration represented by one elapsed-time unit.
    #[must_use]
    #[inline(always)]
    pub const fn time_unit(&self) -> Duration {
        self.time_unit
    }

    /// Returns the configured machine identifier.
    ///
    /// # Returns
    ///
    /// Machine identifier encoded by composed IDs.
    #[must_use]
    #[inline(always)]
    pub const fn machine_id(&self) -> u64 {
        self.machine_id
    }

    /// Returns the maximum representable elapsed time.
    ///
    /// # Returns
    ///
    /// Maximum number of elapsed time units.
    #[must_use]
    #[inline(always)]
    pub const fn max_elapsed_time(&self) -> u64 {
        (1_u64 << self.bits_time) - 1
    }

    /// Returns the maximum representable sequence.
    ///
    /// # Returns
    ///
    /// Maximum sequence number inside one time unit.
    #[must_use]
    #[inline(always)]
    pub const fn max_sequence(&self) -> u64 {
        (1_u64 << self.bits_sequence) - 1
    }

    /// Returns the maximum representable machine identifier.
    ///
    /// # Returns
    ///
    /// Maximum machine identifier.
    #[must_use]
    #[inline(always)]
    pub const fn max_machine_id(&self) -> u64 {
        (1_u64 << self.bits_machine) - 1
    }

    /// Calculates this layout's exclusive expiration for an epoch.
    ///
    /// Elapsed-time values from zero through [`Self::max_elapsed_time`] are
    /// valid. The returned time is the first instant outside that range.
    ///
    /// # Parameters
    ///
    /// * `epoch` - Timestamp origin represented by elapsed time zero.
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
        expiration_time(epoch, self.time_unit, self.max_elapsed_time())
    }

    /// Composes a Sonyflake ID from elapsed-time and sequence parts.
    ///
    /// Repeating the same layout and parts repeats the ID, so this method does
    /// not provide a uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `elapsed_time` - Time units elapsed since the generator epoch.
    /// * `sequence` - Sequence number inside the elapsed-time unit.
    ///
    /// # Returns
    ///
    /// Encoded Sonyflake ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] or
    /// [`IdGenerationError::SequenceOverflow`] when a part exceeds its field.
    pub fn compose_raw(
        &self,
        elapsed_time: u64,
        sequence: u64,
    ) -> Result<u64, IdGenerationError> {
        if elapsed_time > self.max_elapsed_time() {
            return Err(IdGenerationError::TimestampOverflow {
                timestamp: elapsed_time,
                max: self.max_elapsed_time(),
            });
        }
        if sequence > self.max_sequence() {
            return Err(IdGenerationError::SequenceOverflow {
                sequence,
                max: self.max_sequence(),
            });
        }
        Ok((elapsed_time << (self.bits_sequence + self.bits_machine))
            | (sequence << self.bits_machine)
            | self.machine_id)
    }

    /// Decodes a Sonyflake ID using this layout.
    ///
    /// Decoding only extracts fields according to this layout. It does not
    /// authenticate the value or prove that a generator produced it.
    ///
    /// # Parameters
    ///
    /// * `id` - Sonyflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// Elapsed-time, sequence, and machine fields decoded from `id`.
    #[inline]
    pub const fn decode(&self, id: Id) -> SonyflakeParts {
        self.decode_raw(id.value())
    }

    /// Decodes a Sonyflake bit pattern using this layout.
    ///
    /// # Parameters
    ///
    /// * `id` - Sonyflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// Elapsed-time, sequence, and machine fields decoded from `id`.
    #[inline]
    pub const fn decode_raw(&self, id: u64) -> SonyflakeParts {
        let elapsed_time = id >> (self.bits_sequence + self.bits_machine);
        let sequence_mask =
            ((1_u64 << self.bits_sequence) - 1) << self.bits_machine;
        let sequence = (id & sequence_mask) >> self.bits_machine;
        let machine_id = id & ((1_u64 << self.bits_machine) - 1);
        SonyflakeParts::new(elapsed_time, sequence, machine_id)
    }

    /// Composes a Sonyflake ID from elapsed-time and sequence parts.
    ///
    /// # Errors
    ///
    /// Returns the same overflow errors as [`Self::compose_raw`].
    #[inline(always)]
    pub fn compose(
        &self,
        elapsed_time: u64,
        sequence: u64,
    ) -> Result<Id, IdGenerationError> {
        self.compose_raw(elapsed_time, sequence).map(Id::from)
    }

    /// Normalizes and validates one configurable field width.
    ///
    /// # Parameters
    ///
    /// * `name` - Setting name used in errors.
    /// * `bits` - Requested width, or zero for the default.
    /// * `default_bits` - Width selected when `bits` is zero.
    ///
    /// # Returns
    ///
    /// Normalized field width.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::InvalidBitLength`] when the normalized
    /// width is 31 or greater.
    #[inline]
    fn normalize_bits(
        name: &'static str,
        bits: u8,
        default_bits: u8,
    ) -> Result<u8, IdGenerationError> {
        let normalized = if bits == 0 { default_bits } else { bits };
        if normalized >= 31 {
            return Err(IdGenerationError::InvalidBitLength {
                name,
                bits: normalized,
                reason: "bit length must be less than 31",
            });
        }
        Ok(normalized)
    }
}

impl SnowflakeLayoutSpec for SonyflakeLayout {
    /// Returns the configured Sonyflake time unit.
    ///
    /// # Returns
    ///
    /// The duration of one encoded elapsed-time unit.
    #[inline(always)]
    fn time_unit(&self) -> Duration {
        SonyflakeLayout::time_unit(self)
    }

    /// Returns the greatest Sonyflake elapsed-time value.
    ///
    /// # Returns
    ///
    /// The maximum encoded elapsed-time value.
    #[inline(always)]
    fn max_timestamp(&self) -> u64 {
        self.max_elapsed_time()
    }

    /// Returns the greatest Sonyflake sequence value.
    ///
    /// # Returns
    ///
    /// The maximum sequence within one elapsed-time unit.
    #[inline(always)]
    fn max_sequence(&self) -> u64 {
        SonyflakeLayout::max_sequence(self)
    }

    /// Composes a Sonyflake ID.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Elapsed-time value relative to the configured origin.
    /// * `sequence` - Sequence allocated within the elapsed-time unit.
    ///
    /// # Returns
    ///
    /// The composed Sonyflake ID.
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
        SonyflakeLayout::compose_raw(self, timestamp, sequence)
    }
}

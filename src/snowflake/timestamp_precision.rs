/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Timestamp precision for Qubit snowflake IDs.

use super::constants::{
    SEQUENCE_BITS_IN_MILLISECOND,
    SEQUENCE_BITS_IN_SECOND,
    TIMESTAMP_BITS_IN_MILLISECOND,
    TIMESTAMP_BITS_IN_SECOND,
    WAIT_DURATION_IN_MILLISECOND,
    WAIT_DURATION_IN_SECOND,
};

/// Timestamp precision encoded in a Qubit snowflake ID.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TimestampPrecision {
    /// Millisecond precision with 41 timestamp bits and 12 sequence bits.
    Millisecond,
    /// Second precision with 31 timestamp bits and 22 sequence bits.
    Second,
}

impl TimestampPrecision {
    /// Returns the one-bit ordinal used by the Qubit layout.
    ///
    /// # Returns
    /// `0` for millisecond precision and `1` for second precision.
    pub const fn ordinal(self) -> u64 {
        match self {
            Self::Millisecond => 0,
            Self::Second => 1,
        }
    }

    /// Decodes timestamp precision from a one-bit value.
    ///
    /// # Parameters
    /// - `bit`: Encoded one-bit precision value.
    ///
    /// # Returns
    /// [`TimestampPrecision::Millisecond`] for `0`; [`TimestampPrecision::Second`]
    /// for every non-zero value after masking by callers.
    pub const fn from_bit(bit: u64) -> Self {
        if bit == 0 { Self::Millisecond } else { Self::Second }
    }

    /// Returns the number of timestamp bits for this precision.
    ///
    /// # Returns
    /// Timestamp bit length.
    pub const fn timestamp_bits(self) -> u8 {
        match self {
            Self::Millisecond => TIMESTAMP_BITS_IN_MILLISECOND,
            Self::Second => TIMESTAMP_BITS_IN_SECOND,
        }
    }

    /// Returns the number of sequence bits for this precision.
    ///
    /// # Returns
    /// Sequence bit length.
    pub const fn sequence_bits(self) -> u8 {
        match self {
            Self::Millisecond => SEQUENCE_BITS_IN_MILLISECOND,
            Self::Second => SEQUENCE_BITS_IN_SECOND,
        }
    }

    /// Returns the time unit divisor in milliseconds.
    ///
    /// # Returns
    /// `1` for millisecond precision and `1000` for second precision.
    pub const fn divisor_millis(self) -> u64 {
        match self {
            Self::Millisecond => 1,
            Self::Second => 1_000,
        }
    }

    /// Returns the sleep duration used while waiting for a new time slice.
    ///
    /// # Returns
    /// Wait duration in milliseconds.
    pub const fn wait_duration_millis(self) -> u64 {
        match self {
            Self::Millisecond => WAIT_DURATION_IN_MILLISECOND,
            Self::Second => WAIT_DURATION_IN_SECOND,
        }
    }
}

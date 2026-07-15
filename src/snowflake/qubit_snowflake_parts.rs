// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Decoded fields of a Qubit snowflake ID.

use super::{
    IdMode,
    TimestampPrecision,
};

/// Fields decoded from a Qubit snowflake ID.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct QubitSnowflakeParts {
    mode: IdMode,
    precision: TimestampPrecision,
    timestamp: u64,
    host: u64,
    sequence: u64,
}

impl QubitSnowflakeParts {
    /// Creates decoded Qubit snowflake parts.
    ///
    /// # Parameters
    /// - `mode`: Decoded ordering mode.
    /// - `precision`: Decoded timestamp precision.
    /// - `timestamp`: Decoded timestamp.
    /// - `host`: Decoded host identifier.
    /// - `sequence`: Decoded sequence number.
    ///
    /// # Returns
    /// A decoded parts value.
    #[inline]
    pub(crate) const fn new(
        mode: IdMode,
        precision: TimestampPrecision,
        timestamp: u64,
        host: u64,
        sequence: u64,
    ) -> Self {
        Self {
            mode,
            precision,
            timestamp,
            host,
            sequence,
        }
    }

    /// Returns the encoded ID ordering mode.
    ///
    /// # Returns
    /// ID ordering mode.
    #[inline(always)]
    pub const fn mode(self) -> IdMode {
        self.mode
    }

    /// Returns the encoded timestamp precision.
    ///
    /// # Returns
    /// Timestamp precision.
    #[inline(always)]
    pub const fn precision(self) -> TimestampPrecision {
        self.precision
    }

    /// Returns the timestamp measured in the encoded precision.
    ///
    /// # Returns
    /// Timestamp since the generator epoch.
    #[inline(always)]
    pub const fn timestamp(self) -> u64 {
        self.timestamp
    }

    /// Returns the encoded host identifier.
    ///
    /// # Returns
    /// Host identifier.
    #[inline(always)]
    pub const fn host(self) -> u64 {
        self.host
    }

    /// Returns the sequence number inside the timestamp slice.
    ///
    /// # Returns
    /// Sequence number.
    #[inline(always)]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }
}

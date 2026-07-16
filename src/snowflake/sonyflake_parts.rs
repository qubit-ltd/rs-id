// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Decoded fields from a Sonyflake ID.

/// Fields decoded from a configured Sonyflake bit layout.
///
/// Decoding only extracts fields according to the layout. It does not
/// authenticate the value or prove that a generator produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SonyflakeParts {
    /// Time units elapsed since the generator start time.
    elapsed_time: u64,
    /// Sequence number encoded in the ID.
    sequence: u64,
    /// Machine identifier encoded in the ID.
    machine_id: u64,
}

impl SonyflakeParts {
    /// Creates decoded Sonyflake parts.
    ///
    /// # Arguments
    ///
    /// * `elapsed_time` - Time units elapsed since the generator start time.
    /// * `sequence` - Sequence number encoded in the ID.
    /// * `machine_id` - Machine identifier encoded in the ID.
    ///
    /// # Returns
    ///
    /// Parts containing the supplied fields.
    #[inline]
    pub(crate) const fn new(
        elapsed_time: u64,
        sequence: u64,
        machine_id: u64,
    ) -> Self {
        Self {
            elapsed_time,
            sequence,
            machine_id,
        }
    }

    /// Returns the decoded elapsed time.
    ///
    /// # Returns
    ///
    /// Time units elapsed since the generator start time.
    #[must_use]
    #[inline(always)]
    pub const fn elapsed_time(self) -> u64 {
        self.elapsed_time
    }

    /// Returns the decoded sequence number.
    ///
    /// # Returns
    ///
    /// Sequence number encoded in the ID.
    #[must_use]
    #[inline(always)]
    pub const fn sequence(self) -> u64 {
        self.sequence
    }

    /// Returns the decoded machine identifier.
    ///
    /// # Returns
    ///
    /// Machine identifier encoded in the ID.
    #[must_use]
    #[inline(always)]
    pub const fn machine_id(self) -> u64 {
        self.machine_id
    }
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Decoded fields from a classic Snowflake ID.

/// Fields decoded from a classic 41/10/12 Snowflake ID.
///
/// Decoding only extracts fields according to the fixed bit layout. It does
/// not authenticate the value or prove that a generator produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct ClassicalSnowflakeParts {
    /// Milliseconds elapsed since the generator epoch.
    timestamp: u64,
    /// Node identifier encoded in the ID.
    node_id: u64,
    /// Sequence number encoded in the ID.
    sequence: u64,
}

impl ClassicalSnowflakeParts {
    /// Creates decoded classic Snowflake parts.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Milliseconds elapsed since the generator epoch.
    /// * `node_id` - Node identifier encoded in the ID.
    /// * `sequence` - Sequence number encoded in the ID.
    ///
    /// # Returns
    ///
    /// Parts containing the supplied fields.
    #[inline]
    pub(crate) const fn new(
        timestamp: u64,
        node_id: u64,
        sequence: u64,
    ) -> Self {
        Self {
            timestamp,
            node_id,
            sequence,
        }
    }

    /// Returns the decoded timestamp.
    ///
    /// # Returns
    ///
    /// Milliseconds elapsed since the generator epoch.
    #[must_use]
    #[inline(always)]
    pub const fn timestamp(self) -> u64 {
        self.timestamp
    }

    /// Returns the decoded node identifier.
    ///
    /// # Returns
    ///
    /// Node identifier encoded in the ID.
    #[must_use]
    #[inline(always)]
    pub const fn node_id(self) -> u64 {
        self.node_id
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
}

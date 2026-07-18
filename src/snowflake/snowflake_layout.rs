// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stateless classic Snowflake bit layout.

use std::time::{
    Duration,
    SystemTime,
};

use super::internal::{
    SnowflakeLayoutSpec,
    expiration_time,
};
use super::snowflake_parts::SnowflakeParts;
use crate::IdError;

/// Number of bits used for the timestamp field.
const TIMESTAMP_BITS: u8 = 41;
/// Number of bits used for the node field.
const NODE_BITS: u8 = 10;
/// Number of bits used for the sequence field.
const SEQUENCE_BITS: u8 = 12;
/// Maximum node identifier accepted by the layout.
const MAX_NODE_ID: u64 = (1_u64 << NODE_BITS) - 1;

/// Immutable classic 41/10/12 Snowflake bit layout.
///
/// The layout owns the node identifier used by [`Self::compose`]. Composing
/// and decoding are stateless bit operations and provide no uniqueness
/// guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct SnowflakeLayout {
    /// Node identifier encoded by composed IDs.
    node_id: u64,
}

impl SnowflakeLayout {
    /// Creates a classic Snowflake layout for `node_id`.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier in `0..=1023`.
    ///
    /// # Returns
    ///
    /// A configured classic Snowflake layout.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] when `node_id` does not fit the
    /// 10-bit node field.
    #[inline]
    pub fn new(node_id: u64) -> Result<Self, IdError> {
        if node_id > MAX_NODE_ID {
            return Err(IdError::NodeOutOfRange {
                node_id,
                max: MAX_NODE_ID,
            });
        }
        Ok(Self { node_id })
    }

    /// Returns the configured node identifier.
    ///
    /// # Returns
    ///
    /// Node identifier encoded by composed IDs.
    #[must_use]
    #[inline(always)]
    pub const fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Returns the maximum representable timestamp.
    ///
    /// # Returns
    ///
    /// Maximum milliseconds elapsed since the generator epoch.
    #[must_use]
    #[inline(always)]
    pub const fn max_timestamp(&self) -> u64 {
        (1_u64 << TIMESTAMP_BITS) - 1
    }

    /// Returns the maximum representable sequence.
    ///
    /// # Returns
    ///
    /// Maximum sequence number inside one millisecond.
    #[must_use]
    #[inline(always)]
    pub const fn max_sequence(&self) -> u64 {
        (1_u64 << SEQUENCE_BITS) - 1
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
    /// Returns [`IdError::ExpirationTimeOverflow`] when the boundary cannot be
    /// represented by [`SystemTime`].
    #[inline(always)]
    pub fn expires_at(&self, epoch: SystemTime) -> Result<SystemTime, IdError> {
        expiration_time(epoch, Duration::from_millis(1), self.max_timestamp())
    }

    /// Composes a classic Snowflake ID from timestamp and sequence parts.
    ///
    /// Repeating the same layout and parts repeats the ID, so this method does
    /// not provide a uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Milliseconds elapsed since the generator epoch.
    /// * `sequence` - Sequence number inside the timestamp millisecond.
    ///
    /// # Returns
    ///
    /// Encoded classic Snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimestampOverflow`] or
    /// [`IdError::SequenceOverflow`] when a part exceeds its field.
    pub fn compose(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<u64, IdError> {
        if timestamp > self.max_timestamp() {
            return Err(IdError::TimestampOverflow {
                timestamp,
                max: self.max_timestamp(),
            });
        }
        if sequence > self.max_sequence() {
            return Err(IdError::SequenceOverflow {
                sequence,
                max: self.max_sequence(),
            });
        }
        Ok((timestamp << (NODE_BITS + SEQUENCE_BITS))
            | (self.node_id << SEQUENCE_BITS)
            | sequence)
    }

    /// Decodes a classic Snowflake ID.
    ///
    /// Decoding only extracts fields from the fixed layout. It does not
    /// authenticate the value or prove that a generator produced it.
    ///
    /// # Parameters
    ///
    /// * `id` - Classic Snowflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// Timestamp, node, and sequence fields decoded from `id`.
    #[inline]
    pub const fn decode(id: u64) -> SnowflakeParts {
        let timestamp = id >> (NODE_BITS + SEQUENCE_BITS);
        let node_id = (id >> SEQUENCE_BITS) & MAX_NODE_ID;
        let sequence = id & ((1_u64 << SEQUENCE_BITS) - 1);
        SnowflakeParts::new(timestamp, node_id, sequence)
    }
}

impl SnowflakeLayoutSpec for SnowflakeLayout {
    /// Returns the one-millisecond classic Snowflake time unit.
    #[inline(always)]
    fn time_unit(&self) -> Duration {
        Duration::from_millis(1)
    }

    /// Returns the greatest classic Snowflake timestamp.
    #[inline(always)]
    fn max_timestamp(&self) -> u64 {
        SnowflakeLayout::max_timestamp(self)
    }

    /// Returns the greatest classic Snowflake sequence.
    #[inline(always)]
    fn max_sequence(&self) -> u64 {
        SnowflakeLayout::max_sequence(self)
    }

    /// Composes a classic Snowflake ID.
    #[inline(always)]
    fn compose(&self, timestamp: u64, sequence: u64) -> Result<u64, IdError> {
        SnowflakeLayout::compose(self, timestamp, sequence)
    }
}

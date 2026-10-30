// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Stateless classic Snowflake bit layout.

use std::time::Duration;
use std::time::SystemTime;

use super::super::internal::SnowflakeLayoutSpec;
use super::super::internal::expiration_time;
use super::classical_snowflake_parts::ClassicalSnowflakeParts;
use crate::Id;
use crate::IdGenerationError;

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
///
/// # Examples
///
/// ```
/// use qubit_id::ClassicalSnowflakeLayout;
///
/// let layout = ClassicalSnowflakeLayout::new(42)?;
/// let id = layout.compose(1_000, 7)?;
/// let parts = ClassicalSnowflakeLayout::decode(id);
/// assert_eq!(parts.node_id(), 42);
/// assert_eq!(parts.timestamp(), 1_000);
/// assert_eq!(parts.sequence(), 7);
/// # Ok::<(), qubit_id::IdGenerationError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct ClassicalSnowflakeLayout {
    /// Node identifier encoded by composed IDs.
    node_id: u64,
}

impl ClassicalSnowflakeLayout {
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
    /// Returns [`IdGenerationError::NodeOutOfRange`] when `node_id` does not
    /// fit the 10-bit node field.
    #[inline]
    pub fn new(node_id: u64) -> Result<Self, IdGenerationError> {
        if node_id > MAX_NODE_ID {
            return Err(IdGenerationError::NodeOutOfRange {
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
    #[inline]
    pub const fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Returns the maximum representable timestamp.
    ///
    /// # Returns
    ///
    /// Maximum milliseconds elapsed since the generator epoch.
    #[must_use]
    #[inline]
    pub const fn max_timestamp(&self) -> u64 {
        (1_u64 << TIMESTAMP_BITS) - 1
    }

    /// Returns the maximum representable sequence.
    ///
    /// # Returns
    ///
    /// Maximum sequence number inside one millisecond.
    #[must_use]
    #[inline]
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
    /// Returns [`IdGenerationError::ExpirationTimeOverflow`] when the boundary
    /// cannot be represented by [`SystemTime`].
    #[inline]
    pub fn expires_at(&self, epoch: SystemTime) -> Result<SystemTime, IdGenerationError> {
        expiration_time(epoch, Duration::from_millis(1), self.max_timestamp())
    }

    /// Composes a raw classic Snowflake value from timestamp and sequence
    /// parts.
    ///
    /// The timestamp is milliseconds elapsed from the epoch selected by the
    /// caller. The layout supplies the configured node identity and does not
    /// select or retain an epoch or allocation state. This operation is
    /// stateless, so the same inputs produce the same ID and this method does
    /// not provide a uniqueness guarantee.
    ///
    /// Use a generator allocation method to obtain a timestamp and sequence
    /// suitable for a new ID.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Milliseconds elapsed since the caller-selected epoch.
    /// * `sequence` - Sequence from zero through [`Self::max_sequence`],
    ///   inclusive.
    ///
    /// # Returns
    ///
    /// The encoded unsigned 64-bit classic Snowflake value.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] or
    /// [`IdGenerationError::SequenceOverflow`] when a part exceeds its field.
    pub fn compose_raw(&self, timestamp: u64, sequence: u64) -> Result<u64, IdGenerationError> {
        if timestamp > self.max_timestamp() {
            return Err(IdGenerationError::TimestampOverflow {
                timestamp,
                max: self.max_timestamp(),
            });
        }
        if sequence > self.max_sequence() {
            return Err(IdGenerationError::SequenceOverflow {
                sequence,
                max: self.max_sequence(),
            });
        }
        Ok((timestamp << (NODE_BITS + SEQUENCE_BITS)) | (self.node_id << SEQUENCE_BITS) | sequence)
    }

    /// Decodes a classic Snowflake ID.
    ///
    /// The valid encoded range is `0..(1 << 63)`. Decoding only extracts
    /// fields from the fixed layout; it does not reject values with the high
    /// bit set, authenticate the value, or prove that a generator produced it.
    /// Such values can produce a timestamp larger than this layout can
    /// compose. Use [`Self::compose`] when field-range validation is needed.
    ///
    /// # Parameters
    ///
    /// * `id` - Classic Snowflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// Timestamp, node, and sequence fields decoded from `id`.
    #[inline]
    pub const fn decode(id: Id) -> ClassicalSnowflakeParts {
        Self::decode_raw(id.value())
    }

    /// Decodes a classic Snowflake bit pattern.
    ///
    /// The valid encoded range is `0..(1 << 63)`. This method extracts fields
    /// without validating that range; when the high bit is set, the returned
    /// timestamp can exceed [`Self::max_timestamp`]. It does not authenticate
    /// the value or prove that a generator produced it.
    ///
    /// # Parameters
    ///
    /// * `id` - Classic Snowflake bit pattern to decode.
    ///
    /// # Returns
    ///
    /// Timestamp, node, and sequence fields decoded from `id`.
    #[inline]
    pub const fn decode_raw(id: u64) -> ClassicalSnowflakeParts {
        let timestamp = id >> (NODE_BITS + SEQUENCE_BITS);
        let node_id = (id >> SEQUENCE_BITS) & MAX_NODE_ID;
        let sequence = id & ((1_u64 << SEQUENCE_BITS) - 1);
        ClassicalSnowflakeParts::new(timestamp, node_id, sequence)
    }

    /// Composes a typed classic Snowflake [`Id`] from timestamp and sequence
    /// parts.
    ///
    /// The timestamp is milliseconds elapsed from the epoch selected by the
    /// caller. The layout supplies the configured node identity and does not
    /// select or retain an epoch or allocation state. This operation is
    /// stateless, so the same inputs produce the same ID. Use a generator
    /// allocation method to obtain a timestamp and sequence suitable for a new
    /// ID.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Milliseconds elapsed since the caller-selected epoch.
    /// * `sequence` - Sequence from zero through [`Self::max_sequence`],
    ///   inclusive.
    ///
    /// # Returns
    ///
    /// The typed [`Id`] composed with this layout's configured node identity.
    /// Repeating the same layout and parts repeats the ID, so this method does
    /// not provide a uniqueness guarantee.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] when `timestamp` is
    /// greater than [`Self::max_timestamp`], or
    /// [`IdGenerationError::SequenceOverflow`] when `sequence` is greater than
    /// [`Self::max_sequence`].
    #[inline]
    pub fn compose(&self, timestamp: u64, sequence: u64) -> Result<Id, IdGenerationError> {
        self.compose_raw(timestamp, sequence).map(Id::from)
    }
}

impl SnowflakeLayoutSpec for ClassicalSnowflakeLayout {
    /// Returns the one-millisecond classic Snowflake time unit.
    ///
    /// # Returns
    ///
    /// A duration of one millisecond.
    #[inline]
    fn time_unit(&self) -> Duration {
        Duration::from_millis(1)
    }

    /// Returns the greatest classic Snowflake timestamp.
    ///
    /// # Returns
    ///
    /// The maximum encoded millisecond timestamp.
    #[inline]
    fn max_timestamp(&self) -> u64 {
        ClassicalSnowflakeLayout::max_timestamp(self)
    }

    /// Returns the greatest classic Snowflake sequence.
    ///
    /// # Returns
    ///
    /// The maximum sequence within one millisecond.
    #[inline]
    fn max_sequence(&self) -> u64 {
        ClassicalSnowflakeLayout::max_sequence(self)
    }

    /// Composes a classic Snowflake ID.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Milliseconds elapsed since the configured epoch.
    /// * `sequence` - Sequence allocated within the millisecond.
    ///
    /// # Returns
    ///
    /// The composed classic Snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] or
    /// [`IdGenerationError::SequenceOverflow`] when a value exceeds its field.
    #[inline]
    fn compose(&self, timestamp: u64, sequence: u64) -> Result<u64, IdGenerationError> {
        ClassicalSnowflakeLayout::compose_raw(self, timestamp, sequence)
    }
}

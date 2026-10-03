// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private layout contract shared by Snowflake-family generators.

use std::time::Duration;

use crate::IdGenerationError;

/// Supplies the time and bit operations required by the shared allocator.
///
/// Implementations must be safe to share across threads because generators
/// share their layouts while synchronizing allocation state.
pub(crate) trait SnowflakeLayoutSpec: Send + Sync {
    /// Returns the non-zero duration represented by one encoded timestamp unit.
    ///
    /// # Returns
    ///
    /// The duration of one encoded timestamp unit.
    #[must_use]
    fn time_unit(&self) -> Duration;

    /// Returns the greatest encoded timestamp accepted by the layout.
    ///
    /// # Returns
    ///
    /// The maximum encoded timestamp.
    #[must_use]
    fn max_timestamp(&self) -> u64;

    /// Returns the greatest sequence accepted within one timestamp unit.
    ///
    /// # Returns
    ///
    /// The maximum sequence within one timestamp unit.
    #[must_use]
    fn max_sequence(&self) -> u64;

    /// Composes an identifier from an encoded timestamp and sequence.
    ///
    /// # Parameters
    ///
    /// * `timestamp` - Encoded timestamp relative to the configured origin.
    /// * `sequence` - Sequence allocated within the timestamp unit.
    ///
    /// # Returns
    ///
    /// The composed numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimestampOverflow`] when `timestamp`
    /// exceeds the layout capacity or
    /// [`IdGenerationError::SequenceOverflow`] when `sequence` exceeds
    /// its field capacity.
    fn compose(&self, timestamp: u64, sequence: u64) -> Result<u64, IdGenerationError>;
}

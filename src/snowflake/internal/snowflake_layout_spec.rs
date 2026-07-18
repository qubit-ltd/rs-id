// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Private layout contract shared by Snowflake-family generators.

use std::time::Duration;

use crate::IdError;

/// Supplies the time and bit operations required by the shared allocator.
pub(crate) trait SnowflakeLayoutSpec: Send + Sync {
    /// Returns the duration represented by one encoded timestamp unit.
    fn time_unit(&self) -> Duration;

    /// Returns the greatest encoded timestamp accepted by the layout.
    fn max_timestamp(&self) -> u64;

    /// Returns the greatest sequence accepted within one timestamp unit.
    fn max_sequence(&self) -> u64;

    /// Composes an identifier from an encoded timestamp and sequence.
    ///
    /// # Arguments
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
    /// Returns [`IdError`] when either value exceeds the layout capacity.
    fn compose(&self, timestamp: u64, sequence: u64) -> Result<u64, IdError>;
}

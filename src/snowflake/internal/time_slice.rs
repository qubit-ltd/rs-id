// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines generator state for one logical time slice.

/// Mutable timestamp and sequence pair protected by each generator lock.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TimeSlice {
    /// Encoded logical timestamp.
    pub(crate) timestamp: u64,
    /// Last sequence reserved within the timestamp.
    pub(crate) sequence: u64,
}

impl TimeSlice {
    /// Creates a time slice with sequence zero.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Encoded logical timestamp represented by the slice.
    ///
    /// # Returns
    ///
    /// A new time slice whose first sequence is already reserved.
    #[inline]
    pub(crate) const fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            sequence: 0,
        }
    }
}

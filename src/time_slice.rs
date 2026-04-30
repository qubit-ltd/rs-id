/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Internal generator state for a single time slice.

/// Mutable timestamp and sequence pair protected by each generator lock.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TimeSlice {
    pub(crate) timestamp: u64,
    pub(crate) sequence: u64,
}

impl TimeSlice {
    /// Creates a time slice with sequence zero.
    ///
    /// # Parameters
    /// - `timestamp`: Timestamp represented by the slice.
    ///
    /// # Returns
    /// A new time slice.
    pub(crate) const fn new(timestamp: u64) -> Self {
        Self {
            timestamp,
            sequence: 0,
        }
    }

    /// Creates a time slice with an explicit sequence.
    ///
    /// # Parameters
    /// - `timestamp`: Timestamp represented by the slice.
    /// - `sequence`: Sequence value within the timestamp.
    ///
    /// # Returns
    /// A new time slice.
    pub(crate) const fn with_sequence(timestamp: u64, sequence: u64) -> Self {
        Self {
            timestamp,
            sequence,
        }
    }
}

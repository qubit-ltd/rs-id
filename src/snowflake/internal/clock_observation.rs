// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Defines an unquantized wall-clock observation.

use std::time::{
    Duration,
    SystemTime,
};

use crate::IdError;

/// Raw and quantized time observed during one allocation attempt.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[must_use]
pub(crate) struct ClockObservation {
    /// Unquantized duration since the configured reference time.
    pub(crate) elapsed: Duration,
    /// Encoded logical time slice.
    pub(crate) timestamp: u64,
    /// Positive duration from this observation to the next slice boundary.
    pub(crate) retry_after: Duration,
}

impl ClockObservation {
    /// Converts one wall time into raw and quantized generator time.
    ///
    /// # Parameters
    ///
    /// * `time` - Wall time reported by the generator clock.
    /// * `epoch` - Reference time for the encoded timestamp.
    /// * `time_unit` - Duration represented by one encoded timestamp unit.
    /// * `max_timestamp` - Largest timestamp supported by the layout.
    ///
    /// # Returns
    ///
    /// The raw elapsed duration, encoded timestamp, and exact time remaining
    /// before the next logical slice.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` precedes `epoch`.
    pub(crate) fn from_time(
        time: SystemTime,
        epoch: SystemTime,
        time_unit: Duration,
        max_timestamp: u64,
    ) -> Result<Self, IdError> {
        debug_assert!(!time_unit.is_zero());
        let elapsed = time
            .duration_since(epoch)
            .map_err(|_| IdError::TimeBeforeEpoch { time, epoch })?;
        let unit_nanos = time_unit.as_nanos();
        let timestamp = elapsed.as_nanos() / unit_nanos;
        debug_assert!(timestamp <= u128::from(max_timestamp));
        let elapsed_in_slice = elapsed.as_nanos() % unit_nanos;
        let retry_nanos = unit_nanos - elapsed_in_slice;
        const NANOS_PER_SECOND: u128 = 1_000_000_000;
        let retry_seconds = retry_nanos / NANOS_PER_SECOND;
        debug_assert!(retry_seconds <= u128::from(u64::MAX));
        let retry_after = Duration::new(
            retry_seconds as u64,
            (retry_nanos % NANOS_PER_SECOND) as u32,
        );
        Ok(Self {
            elapsed,
            timestamp: timestamp as u64,
            retry_after,
        })
    }
}

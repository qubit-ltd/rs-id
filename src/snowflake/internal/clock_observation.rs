// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
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
    /// # Arguments
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
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` precedes `epoch`, or
    /// [`IdError::TimestampOverflow`] when the encoded timestamp exceeds the
    /// layout.
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
        if timestamp > u128::from(max_timestamp) {
            return Err(IdError::TimestampOverflow {
                timestamp: u64::try_from(timestamp).unwrap_or(u64::MAX),
                max: max_timestamp,
            });
        }
        let elapsed_in_slice = elapsed.as_nanos() % unit_nanos;
        let retry_after = duration_from_nanos(unit_nanos - elapsed_in_slice);
        Ok(Self {
            elapsed,
            timestamp: timestamp as u64,
            retry_after,
        })
    }
}

/// Converts a representable nanosecond count to a duration.
///
/// # Arguments
///
/// * `nanos` - Nanosecond count known to fit in [`Duration`].
///
/// # Returns
///
/// The equivalent duration.
#[must_use]
#[inline]
fn duration_from_nanos(nanos: u128) -> Duration {
    const NANOS_PER_SECOND: u128 = 1_000_000_000;
    let seconds = nanos / NANOS_PER_SECOND;
    debug_assert!(seconds <= u128::from(u64::MAX));
    Duration::new(seconds as u64, (nanos % NANOS_PER_SECOND) as u32)
}

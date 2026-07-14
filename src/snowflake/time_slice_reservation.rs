// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Shared reservation transition for Snowflake-family generators.

use super::time_slice::TimeSlice;

/// Result of attempting to reserve a timestamp and sequence pair.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TimeSliceReservation {
    /// A timestamp and sequence pair was reserved.
    Allocated(TimeSlice),
    /// Allocation must wait until the clock advances past this timestamp.
    WaitForNext(u64),
    /// The clock is behind the latest timestamp stored in the generator.
    ClockMovedBackwards {
        /// Latest timestamp stored in the generator.
        last_timestamp: u64,
        /// Timestamp currently reported by the clock.
        current_timestamp: u64,
    },
}

/// Reserves the next timestamp and sequence pair from generator state.
///
/// An empty state is initialized with an exhausted sequence so that callers
/// preserve the startup fence by waiting for the next time slice. The state is
/// not changed when the clock moves backwards or a sequence range is
/// exhausted.
///
/// # Parameters
/// - `state`: Mutable generator state protected by the caller's lock.
/// - `current_timestamp`: Timestamp currently reported by the clock.
/// - `max_sequence`: Maximum sequence value for the generator layout.
///
/// # Returns
/// The reserved pair, a timestamp to wait past, or a backwards-clock result.
pub(crate) fn reserve_next(
    state: &mut Option<TimeSlice>,
    current_timestamp: u64,
    max_sequence: u64,
) -> TimeSliceReservation {
    let Some(time_slice) = state.as_mut() else {
        *state =
            Some(TimeSlice::with_sequence(current_timestamp, max_sequence));
        return TimeSliceReservation::WaitForNext(current_timestamp);
    };

    if time_slice.timestamp > current_timestamp {
        return TimeSliceReservation::ClockMovedBackwards {
            last_timestamp: time_slice.timestamp,
            current_timestamp,
        };
    }

    if current_timestamp > time_slice.timestamp {
        *time_slice = TimeSlice::new(current_timestamp);
        return TimeSliceReservation::Allocated(*time_slice);
    }

    if time_slice.sequence == max_sequence {
        return TimeSliceReservation::WaitForNext(time_slice.timestamp);
    }

    time_slice.sequence += 1;
    TimeSliceReservation::Allocated(*time_slice)
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Boundary values shared by integration tests.

use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

/// Finds the latest representable whole-second timestamp.
///
/// # Returns
///
/// The latest [`SystemTime`] on or after [`UNIX_EPOCH`] whose subsecond
/// component is zero.
pub(crate) fn latest_representable_whole_second() -> SystemTime {
    let mut low = 0_u64;
    let mut high = u64::MAX;
    while low < high {
        let difference = high - low;
        let middle = low + difference / 2 + difference % 2;
        if UNIX_EPOCH.checked_add(Duration::from_secs(middle)).is_some() {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    UNIX_EPOCH
        .checked_add(Duration::from_secs(low))
        .expect("binary search must retain a representable time")
}

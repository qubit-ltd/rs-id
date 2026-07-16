// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines summarized generator startup latency.

/// Minimum, median, and maximum build-plus-first-ID latency in nanoseconds.
pub(super) struct StartupLatencySummary {
    /// Lowest observed startup latency in nanoseconds.
    pub(super) min_nanos: u128,
    /// Median observed startup latency in nanoseconds.
    pub(super) median_nanos: u128,
    /// Highest observed startup latency in nanoseconds.
    pub(super) max_nanos: u128,
}

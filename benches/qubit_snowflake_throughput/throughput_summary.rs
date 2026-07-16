// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines summarized sustained-throughput observations.

use super::throughput_sample::ThroughputSample;

/// Minimum, median, and maximum observations ordered by throughput.
pub(super) struct ThroughputSummary {
    /// Observation with the lowest measured throughput.
    pub(super) min: ThroughputSample,
    /// Median observation by measured throughput.
    pub(super) median: ThroughputSample,
    /// Observation with the highest measured throughput.
    pub(super) max: ThroughputSample,
}

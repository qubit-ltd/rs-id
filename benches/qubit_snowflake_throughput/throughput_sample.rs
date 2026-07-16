// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines one sustained-throughput benchmark observation.

use std::time::Duration;

/// One sustained-throughput observation.
#[derive(Clone, Copy)]
pub(super) struct ThroughputSample {
    /// Number of IDs generated in the observation.
    pub(super) generated: u64,
    /// Theoretical sequence capacity for the measured slices.
    pub(super) capacity: u64,
    /// Wall duration of the observation.
    pub(super) elapsed: Duration,
}

impl ThroughputSample {
    /// Returns the percentage of theoretical sequence capacity consumed.
    ///
    /// # Returns
    ///
    /// Generated IDs divided by theoretical capacity, expressed as a
    /// percentage.
    #[inline(always)]
    pub(super) fn utilization(self) -> f64 {
        self.generated as f64 * 100.0 / self.capacity as f64
    }

    /// Returns IDs generated per elapsed second.
    ///
    /// # Returns
    ///
    /// The observation's generated count divided by elapsed seconds.
    #[inline(always)]
    pub(super) fn throughput(self) -> f64 {
        self.generated as f64 / self.elapsed.as_secs_f64()
    }
}

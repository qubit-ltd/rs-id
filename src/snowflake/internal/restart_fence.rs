// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines the first-allocation fence used by restart policies.

use super::super::RestartPolicy;

/// Tracks whether a restarted generator may allocate in the observed slice.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum RestartFence {
    /// Allocation may begin immediately.
    Disabled,
    /// The first timestamp has not yet been observed.
    Uninitialized,
    /// Allocation is waiting for a timestamp after the baseline.
    Waiting {
        /// Timestamp observed by the first generation attempt.
        baseline_timestamp: u64,
    },
}

impl RestartFence {
    /// Creates a fence for the configured restart policy.
    ///
    /// # Arguments
    ///
    /// * `policy` - Restart behavior selected for the generator.
    ///
    /// # Returns
    ///
    /// A disabled fence for immediate allocation, or an uninitialized fence
    /// that will capture the first observed slice.
    #[inline]
    pub(crate) const fn new(policy: RestartPolicy) -> Self {
        match policy {
            RestartPolicy::Immediate => Self::Disabled,
            RestartPolicy::WaitNextSlice => Self::Uninitialized,
        }
    }

    /// Returns whether allocation must still wait at `timestamp`.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Logical time slice observed by the current attempt.
    ///
    /// # Returns
    ///
    /// `true` while the restart fence remains active; otherwise `false`.
    #[inline]
    pub(crate) fn should_wait(&mut self, timestamp: u64) -> bool {
        match *self {
            Self::Disabled => false,
            Self::Uninitialized => {
                *self = Self::Waiting {
                    baseline_timestamp: timestamp,
                };
                true
            }
            Self::Waiting { baseline_timestamp }
                if timestamp <= baseline_timestamp =>
            {
                true
            }
            Self::Waiting { .. } => {
                *self = Self::Disabled;
                false
            }
        }
    }
}

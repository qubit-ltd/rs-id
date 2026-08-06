// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Result of a non-blocking ID allocation attempt.

use std::time::Duration;

/// Result of an allocation attempt that never sleeps or awaits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "generation attempts must be handled"]
pub enum GenerationAttempt<T> {
    /// Allocation completed with a generated value.
    Generated(T),
    /// Allocation can continue after the specified duration.
    RetryAfter {
        /// Minimum duration before another allocation attempt.
        delay: Duration,
    },
}

impl<T> GenerationAttempt<T> {
    /// Converts a generated value while preserving retry decisions.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> GenerationAttempt<U> {
        type Output<U> = GenerationAttempt<U>;

        match self {
            Self::Generated(value) => GenerationAttempt::Generated(f(value)),
            Self::RetryAfter { delay } => Output::<U>::RetryAfter { delay },
        }
    }
}

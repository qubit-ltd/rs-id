// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Internal result of one non-waiting generation attempt.

use std::time::Duration;

/// Result of an allocation attempt that never sleeps or awaits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use = "generation attempts must be handled"]
pub(crate) enum GenerationAttempt<T> {
    /// Allocation completed with a generated value.
    Generated(T),
    /// Allocation must be retried after the specified positive duration.
    RetryAfter(Duration),
}

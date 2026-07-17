// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a timer that always fails registration.

use qubit_clock::{
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
    TimeError,
    Timer,
    TimerFuture,
};

/// Timer used to verify error-source propagation.
pub(crate) struct FailingTimer {
    /// Monotonic clock returned by the timer contract.
    clock: StdMonotonicClock,
}

impl FailingTimer {
    /// Creates a timer that always reports instant overflow.
    ///
    /// # Returns
    ///
    /// A failing timer with its own monotonic clock domain.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            clock: StdMonotonicClock::new(),
        }
    }
}

impl Timer for FailingTimer {
    /// Returns the fixture's monotonic clock.
    ///
    /// # Returns
    ///
    /// The monotonic clock required by the timer contract.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Returns a stable error without registering a deadline.
    ///
    /// # Parameters
    ///
    /// * `_deadline` - Ignored deadline in the fixture's clock domain.
    ///
    /// # Returns
    ///
    /// This fixture never succeeds.
    ///
    /// # Errors
    ///
    /// Always returns [`TimeError::InstantOverflow`].
    #[inline]
    fn at(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<TimerFuture, TimeError> {
        Err(TimeError::InstantOverflow)
    }
}

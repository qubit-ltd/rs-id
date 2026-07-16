// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a blocking sleeper that always fails.

use qubit_clock::{
    BlockingSleeper,
    MonotonicClock,
    MonotonicInstant,
    StdMonotonicClock,
    TimeError,
};

/// Blocking sleeper used to verify error-source propagation.
pub(crate) struct FailingBlockingSleeper {
    /// Monotonic clock returned by the sleeper contract.
    clock: StdMonotonicClock,
}

impl FailingBlockingSleeper {
    /// Creates a sleeper that always reports instant overflow.
    ///
    /// # Returns
    ///
    /// A failing sleeper with its own monotonic clock domain.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            clock: StdMonotonicClock::new(),
        }
    }
}

impl BlockingSleeper for FailingBlockingSleeper {
    /// Returns the fixture's monotonic clock.
    ///
    /// # Returns
    ///
    /// The monotonic clock required by the sleeper contract.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Returns a stable error without blocking.
    ///
    /// # Arguments
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
    fn sleep_until(
        &self,
        _deadline: MonotonicInstant,
    ) -> Result<(), TimeError> {
        Err(TimeError::InstantOverflow)
    }
}

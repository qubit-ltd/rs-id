// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a timer whose first future fails after successful registration.

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_clock::MonotonicClock;
use qubit_clock::MonotonicInstant;
use qubit_clock::StdMonotonicClock;
use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;
use qubit_clock::TimerUnavailableError;

/// Timer used to verify completion-error propagation after registration.
pub(crate) struct CompletionFailingTimer {
    /// Monotonic clock returned by the timer contract.
    clock: StdMonotonicClock,
    /// Number of deadline registrations attempted by the fixture.
    registrations: AtomicUsize,
}

impl CompletionFailingTimer {
    /// Creates a timer that fails its first registered future.
    ///
    /// # Returns
    ///
    /// A completion-failing timer with its own monotonic clock domain.
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            clock: StdMonotonicClock::new(),
            registrations: AtomicUsize::new(0),
        }
    }
}

impl Timer for CompletionFailingTimer {
    /// Returns the fixture's monotonic clock.
    ///
    /// # Returns
    ///
    /// The monotonic clock required by the timer contract.
    #[inline(always)]
    fn clock(&self) -> &dyn MonotonicClock {
        &self.clock
    }

    /// Registers a future that fails on completion for the first call.
    ///
    /// A second call fails during registration so a consumer that incorrectly
    /// ignores the first future's result exposes a distinguishable error.
    ///
    /// # Parameters
    ///
    /// * `_deadline` - Ignored deadline in the fixture's clock domain.
    ///
    /// # Returns
    ///
    /// A future resolving to scheduler-worker failure on the first call.
    ///
    /// # Errors
    ///
    /// Returns [`TimeError::InstantOverflow`] on every later registration.
    #[inline]
    fn at(&self, _deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        if self.registrations.fetch_add(1, Ordering::Relaxed) == 0 {
            return Ok(Box::pin(async {
                Err(TimeError::TimerUnavailable {
                    source: TimerUnavailableError::SchedulerWorkerTerminated,
                })
            }));
        }
        Err(TimeError::InstantOverflow)
    }
}

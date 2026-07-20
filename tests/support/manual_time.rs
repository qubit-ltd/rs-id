// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines deterministic wall time and timer waits for tests.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use qubit_clock::{
    ManualMonotonicClock,
    ManualWallClock,
    MonotonicClock,
    MonotonicInstant,
    Timer,
    WallClock,
};

/// Couples manual wall time and blocking waits to one monotonic timeline.
pub(crate) struct ManualTime {
    /// Monotonic timeline advanced by the test driver.
    monotonic_clock: Arc<ManualMonotonicClock>,
    /// Wall-time projection used by generators.
    wall_clock: Arc<ManualWallClock>,
    /// Timer registered on the same monotonic timeline.
    timer: Arc<dyn Timer>,
}

impl ManualTime {
    /// Creates a fixture whose wall clock initially reads `now`.
    ///
    /// # Parameters
    ///
    /// * `now` - Initial wall time projected from monotonic time zero.
    ///
    /// # Returns
    ///
    /// A deterministic wall clock and timer pair.
    pub(crate) fn new(now: SystemTime) -> Self {
        let monotonic_clock = ManualMonotonicClock::new_shared();
        let wall_clock = monotonic_clock.new_wall_clock(now);
        let timer = monotonic_clock.new_timer();
        Self {
            monotonic_clock,
            wall_clock,
            timer,
        }
    }

    /// Returns the wall clock as its public trait object.
    ///
    /// # Returns
    ///
    /// A shared wall-clock handle.
    #[inline(always)]
    pub(crate) fn wall_clock(&self) -> Arc<dyn WallClock> {
        self.wall_clock.clone()
    }

    /// Returns the timer as its public trait object.
    ///
    /// # Returns
    ///
    /// A shared timer handle on the manual timeline.
    #[inline(always)]
    pub(crate) fn timer(&self) -> Arc<dyn Timer> {
        Arc::clone(&self.timer)
    }

    /// Reanchors wall time without moving monotonic time.
    ///
    /// # Parameters
    ///
    /// * `now` - New wall time for the current monotonic instant.
    #[inline(always)]
    pub(crate) fn reanchor(&self, now: SystemTime) {
        self.wall_clock.reanchor(now);
    }

    /// Advances both monotonic time and the wall-time projection.
    ///
    /// # Parameters
    ///
    /// * `duration` - Amount by which to advance the shared timeline.
    ///
    /// # Panics
    ///
    /// Panics when advancing the manual monotonic clock overflows.
    #[cfg(feature = "qubit-snowflake")]
    #[inline(always)]
    pub(crate) fn advance(&self, duration: Duration) {
        self.monotonic_clock
            .advance(duration)
            .expect("manual time should advance");
    }

    /// Waits for one timer deadline and advances directly to it.
    ///
    /// # Panics
    ///
    /// Panics when no timer wait registers within the test timeout or advancing
    /// to the registered deadline fails.
    #[inline(always)]
    pub(crate) fn advance_to_next_deadline(&self) {
        self.advance_to_next_deadline_after_waiters(1);
    }

    /// Waits for `expected` timer waiters and advances to their next deadline.
    ///
    /// # Parameters
    ///
    /// * `expected` - Number of registered timer waiters required before time
    ///   advances.
    ///
    /// # Panics
    ///
    /// Panics when the expected waiters do not register within the test
    /// timeout or advancing to their next deadline fails.
    pub(crate) fn advance_to_next_deadline_after_waiters(
        &self,
        expected: usize,
    ) {
        let _ = self
            .monotonic_clock
            .advance_to_next_deadline_after_waiters(
                expected,
                Duration::from_secs(1),
            )
            .unwrap_or_else(|| {
                panic!("generators should register {expected} future deadlines")
            });
    }

    /// Asynchronously waits until a future timer deadline is registered.
    ///
    /// # Returns
    ///
    /// The earliest future deadline registered on the manual timeline.
    #[inline(always)]
    pub(crate) async fn wait_for_next_deadline_async(
        &self,
    ) -> MonotonicInstant {
        self.monotonic_clock.wait_for_next_deadline_async().await
    }

    /// Asynchronously waits for and advances to the next timer deadline.
    ///
    /// # Returns
    ///
    /// The same-domain instant reached on the manual timeline.
    #[inline(always)]
    pub(crate) async fn advance_to_next_deadline_async(
        &self,
    ) -> MonotonicInstant {
        self.monotonic_clock.advance_to_next_deadline_async().await
    }

    /// Asynchronously waits until `expected` timer waiters are registered.
    ///
    /// # Parameters
    ///
    /// * `expected` - Registration count that completes the observation.
    #[cfg(feature = "qubit-snowflake")]
    #[inline(always)]
    pub(crate) async fn wait_for_waiters_async(&self, expected: usize) {
        self.monotonic_clock.wait_for_waiters_async(expected).await;
    }

    /// Returns the number of timer waiter registrations awaiting cleanup.
    ///
    /// # Returns
    ///
    /// The current registration count.
    #[cfg(feature = "qubit-snowflake")]
    #[inline(always)]
    pub(crate) fn pending_waiters(&self) -> usize {
        self.monotonic_clock.pending_waiters()
    }
}

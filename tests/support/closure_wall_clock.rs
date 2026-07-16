// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Adapts a closure to the wall-clock trait for focused test behavior.

use std::time::SystemTime;

use qubit_clock::WallClock;

/// Wall clock backed by a thread-safe test closure of type `F`.
pub(crate) struct ClosureWallClock<F> {
    /// Closure sampled by each wall-clock read.
    now: F,
}

impl<F> ClosureWallClock<F> {
    /// Wraps a closure as a wall clock.
    ///
    /// # Arguments
    ///
    /// * `now` - Closure returning the current test wall time.
    ///
    /// # Returns
    ///
    /// A wall-clock adapter backed by `now`.
    #[inline]
    pub(crate) const fn new(now: F) -> Self {
        Self { now }
    }
}

impl<F> WallClock for ClosureWallClock<F>
where
    F: Fn() -> SystemTime + Send + Sync,
{
    /// Samples the wrapped closure.
    ///
    /// # Returns
    ///
    /// The current test wall time.
    #[inline(always)]
    fn now(&self) -> SystemTime {
        (self.now)()
    }
}

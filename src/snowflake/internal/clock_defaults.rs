// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Constructs default wall-clock and blocking-sleeper capabilities.

use std::sync::Arc;

use qubit_clock::{
    BlockingSleeper,
    StdBlockingSleeper,
    StdWallClock,
    WallClock,
};

/// Creates the standard system wall clock.
///
/// # Returns
///
/// A shared wall-clock trait object backed by [`std::time::SystemTime`].
#[inline]
pub(crate) fn default_wall_clock() -> Arc<dyn WallClock> {
    Arc::new(StdWallClock::new())
}

/// Creates the standard blocking sleeper and its monotonic clock.
///
/// # Returns
///
/// A shared blocking-sleeper trait object backed by standard monotonic time.
#[inline]
pub(crate) fn default_blocking_sleeper() -> Arc<dyn BlockingSleeper> {
    Arc::new(StdBlockingSleeper::new())
}

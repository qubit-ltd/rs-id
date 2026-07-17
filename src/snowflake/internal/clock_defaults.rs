// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Constructs default wall-clock and timer capabilities.

use std::sync::{
    Arc,
    OnceLock,
};

use qubit_clock::{
    MonotonicClock,
    StdMonotonicClock,
    StdWallClock,
    Timer,
    WallClock,
};

/// Process-wide standard timer used by default generator configurations.
static DEFAULT_TIMER: OnceLock<Arc<dyn Timer>> = OnceLock::new();

/// Creates the standard system wall clock.
///
/// # Returns
///
/// A shared wall-clock trait object backed by [`std::time::SystemTime`].
#[must_use]
#[inline(always)]
pub(crate) fn default_wall_clock() -> Arc<dyn WallClock> {
    Arc::new(StdWallClock::new())
}

/// Returns the process-wide standard timer and monotonic clock.
///
/// # Returns
///
/// A shared timer trait object backed by standard monotonic time.
#[must_use]
#[inline(always)]
pub(crate) fn default_timer() -> Arc<dyn Timer> {
    Arc::clone(
        DEFAULT_TIMER.get_or_init(|| StdMonotonicClock::new().new_timer()),
    )
}

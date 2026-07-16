// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Defines a wall clock that panics on one configured sample.

use std::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::time::SystemTime;

use qubit_clock::WallClock;

/// Wall clock used to verify generator recovery after a clock panic.
pub(crate) struct PanickingWallClock {
    /// Zero-based sample index that panics.
    panic_on_call: u64,
    /// Stable wall time returned by every non-panicking sample.
    time: SystemTime,
    /// Number of samples already attempted.
    call_count: AtomicU64,
}

impl PanickingWallClock {
    /// Creates a wall clock that panics on one sample.
    ///
    /// # Arguments
    ///
    /// * `panic_on_call` - Zero-based sample index that should panic.
    /// * `time` - Wall time returned by all other samples.
    ///
    /// # Returns
    ///
    /// A deterministic panicking wall clock.
    #[inline]
    pub(crate) const fn new(panic_on_call: u64, time: SystemTime) -> Self {
        Self {
            panic_on_call,
            time,
            call_count: AtomicU64::new(0),
        }
    }
}

impl WallClock for PanickingWallClock {
    /// Panics at the configured sample and otherwise returns the fixed time.
    ///
    /// # Returns
    ///
    /// The configured stable wall time on non-panicking calls.
    ///
    /// # Panics
    ///
    /// Panics when the current sample index equals `panic_on_call`.
    #[inline]
    fn now(&self) -> SystemTime {
        let call = self.call_count.fetch_add(1, Ordering::SeqCst);
        assert_ne!(call, self.panic_on_call, "test clock panic");
        self.time
    }
}

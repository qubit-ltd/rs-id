// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_clock::ManualMonotonicClock;
use qubit_clock::ManualWallClock;
use qubit_clock::MonotonicClock;
use qubit_clock::MonotonicInstant;
use qubit_clock::TimeError;
use qubit_clock::Timer;
use qubit_clock::TimerFuture;

pub(crate) const REGISTRATION_LIMIT: usize = 64;

pub(crate) fn manual_wall_clock(now: std::time::SystemTime, clock: Arc<ManualMonotonicClock>) -> Arc<ManualWallClock> {
    Arc::new(ManualWallClock::from_clock(now, clock))
}

pub(crate) struct AdvancingFuseTimer {
    clock: Arc<ManualMonotonicClock>,
    registrations: AtomicUsize,
    registration_limit: usize,
    completion_error: bool,
}

impl AdvancingFuseTimer {
    pub(crate) fn new(clock: Arc<ManualMonotonicClock>, registration_limit: usize, completion_error: bool) -> Self {
        Self {
            clock,
            registrations: AtomicUsize::new(0),
            registration_limit,
            completion_error,
        }
    }

    pub(crate) fn registrations(&self) -> usize {
        self.registrations.load(Ordering::SeqCst)
    }
}

impl Timer for AdvancingFuseTimer {
    fn clock(&self) -> &dyn MonotonicClock {
        self.clock.as_ref()
    }

    fn at(&self, deadline: MonotonicInstant) -> Result<TimerFuture, TimeError> {
        deadline.validate_domain(self.clock.domain())?;
        let registration = self.registrations.fetch_add(1, Ordering::SeqCst) + 1;
        if registration >= self.registration_limit {
            return Err(TimeError::InstantOverflow);
        }

        let now = self.clock.now();
        let delay = deadline.duration_since(now)?;
        self.clock.advance(delay)?;
        if self.completion_error {
            Ok(Box::pin(async { Err(TimeError::InstantOverflow) }))
        } else {
            Ok(Box::pin(async { Ok(()) }))
        }
    }
}

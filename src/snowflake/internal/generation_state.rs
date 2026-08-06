// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Defines shared allocation state for Snowflake-family generators.

use std::time::Duration;

use super::{ClockObservation, GenerationAttempt, RestartFence, TimeSlice};
use crate::{IdError, RestartPolicy};

/// Mutable high-water and sequence state protected by a generator lock.
#[derive(Debug)]
#[must_use]
pub(crate) struct GenerationState {
    /// Greatest raw elapsed time observed without rollback.
    last_observed_elapsed: Option<Duration>,
    /// Last allocated logical slice and sequence.
    time_slice: Option<TimeSlice>,
    /// First-allocation restart fence.
    restart_fence: RestartFence,
}

impl GenerationState {
    /// Creates empty allocation state for `restart_policy`.
    ///
    /// # Parameters
    ///
    /// * `restart_policy` - Policy controlling the first allocation.
    ///
    /// # Returns
    ///
    /// Empty state with a restart fence configured for the policy.
    #[inline]
    pub(crate) const fn new(restart_policy: RestartPolicy) -> Self {
        Self {
            last_observed_elapsed: None,
            time_slice: None,
            restart_fence: RestartFence::new(restart_policy),
        }
    }

    /// Reserves a timestamp and sequence without blocking.
    ///
    /// Raw clock rollback is checked before logical timestamps are compared.
    /// A tolerated rollback leaves all state unchanged so allocation cannot
    /// resume until wall time catches up to the greatest observed value.
    ///
    /// # Parameters
    ///
    /// * `observation` - Raw and quantized wall-clock observation.
    /// * `max_sequence` - Largest sequence supported by the layout.
    /// * `max_clock_skew` - Maximum tolerated raw clock rollback.
    ///
    /// # Returns
    ///
    /// A reserved time slice or the exact duration before retrying.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::ClockMovedBackwards`] when raw rollback exceeds the
    /// configured tolerance.
    pub(crate) fn reserve(
        &mut self,
        observation: ClockObservation,
        max_sequence: u64,
        max_clock_skew: Duration,
    ) -> Result<GenerationAttempt<TimeSlice>, IdError> {
        if let Some(last_elapsed) = self.last_observed_elapsed
            && observation.elapsed < last_elapsed
        {
            let skew = last_elapsed - observation.elapsed;
            if skew > max_clock_skew {
                return Err(IdError::ClockMovedBackwards {
                    last_elapsed,
                    current_elapsed: observation.elapsed,
                    skew,
                    max_skew: max_clock_skew,
                });
            }
            return Ok(GenerationAttempt::RetryAfter { delay: skew });
        }
        self.last_observed_elapsed = Some(observation.elapsed);

        if self.restart_fence.should_wait(observation.timestamp) {
            return Ok(GenerationAttempt::RetryAfter {
                delay: observation.retry_after,
            });
        }

        let Some(time_slice) = self.time_slice.as_mut() else {
            let time_slice = TimeSlice::new(observation.timestamp);
            self.time_slice = Some(time_slice);
            return Ok(GenerationAttempt::Generated(time_slice));
        };
        if observation.timestamp > time_slice.timestamp {
            *time_slice = TimeSlice::new(observation.timestamp);
            return Ok(GenerationAttempt::Generated(*time_slice));
        }
        debug_assert_eq!(observation.timestamp, time_slice.timestamp);
        if time_slice.sequence == max_sequence {
            return Ok(GenerationAttempt::RetryAfter {
                delay: observation.retry_after,
            });
        }
        time_slice.sequence += 1;
        Ok(GenerationAttempt::Generated(*time_slice))
    }
}

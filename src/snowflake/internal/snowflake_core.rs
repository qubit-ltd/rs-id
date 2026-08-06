// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
// qubit-style: allow source-test-pair
//! Shared, non-waiting Snowflake allocation core.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use parking_lot::Mutex;
use qubit_clock::WallClock;

use super::{
    ClockObservation,
    GenerationAttempt,
    GenerationState,
    SnowflakeLayoutSpec,
};
use crate::{
    IdError,
    RestartPolicy,
};

/// Owns the shared Snowflake layout, clock, and synchronized allocation state.
pub(crate) struct SnowflakeCore<L> {
    /// Bit layout used to compose generated IDs.
    layout: L,
    /// Timestamp origin used by encoded timestamps.
    epoch: SystemTime,
    /// Exclusive timestamp expiration boundary.
    expires_at: SystemTime,
    /// Maximum tolerated raw wall-clock rollback.
    max_clock_skew: Duration,
    /// Wall clock sampled inside the allocation critical section.
    wall_clock: Arc<dyn WallClock>,
    /// Synchronized raw-clock and sequence allocation state.
    state: Mutex<GenerationState>,
}

impl<L> SnowflakeCore<L>
where
    L: SnowflakeLayoutSpec,
{
    /// Creates a core from a validated generator configuration.
    ///
    /// # Parameters
    ///
    /// * `layout` - Validated Snowflake-family layout.
    /// * `epoch` - Timestamp origin used by the layout.
    /// * `expires_at` - Exclusive generator expiration boundary.
    /// * `max_clock_skew` - Largest tolerated raw wall-clock rollback.
    /// * `restart_policy` - Policy controlling the first allocation.
    /// * `wall_clock` - Wall clock sampled by allocation attempts.
    ///
    /// # Returns
    ///
    /// A core containing the complete non-waiting allocation state.
    #[inline]
    pub(crate) fn new(
        layout: L,
        epoch: SystemTime,
        expires_at: SystemTime,
        max_clock_skew: Duration,
        restart_policy: RestartPolicy,
        wall_clock: Arc<dyn WallClock>,
    ) -> Self {
        Self {
            layout,
            epoch,
            expires_at,
            max_clock_skew,
            wall_clock,
            state: Mutex::new(GenerationState::new(restart_policy)),
        }
    }

    /// Performs one allocation attempt without waiting.
    ///
    /// Wall time is sampled while the state lock is held so concurrent callers
    /// cannot observe time in one order and mutate allocation state in another.
    /// The returned retry delay must be waited outside this method.
    ///
    /// # Returns
    ///
    /// A generated ID or a positive retry delay.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when the clock precedes the epoch,
    /// [`IdError::GeneratorExpired`] at the lifetime boundary, or
    /// [`IdError::ClockMovedBackwards`] when rollback exceeds the configured
    /// tolerance.
    pub(crate) fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<u64>, IdError> {
        let mut state = self.state.lock();
        let observed_at = self.wall_clock.now();
        self.ensure_active(observed_at)?;
        let observation = self.observation_for(observed_at)?;
        match state.reserve(
            observation,
            self.layout.max_sequence(),
            self.max_clock_skew,
        )? {
            GenerationAttempt::Generated(time_slice) => self
                .layout
                .compose(time_slice.timestamp, time_slice.sequence)
                .map(GenerationAttempt::Generated),
            GenerationAttempt::RetryAfter { delay } => {
                Ok(GenerationAttempt::RetryAfter { delay })
            }
        }
    }

    /// Composes an ID for an explicit wall time and sequence.
    ///
    /// This operation is stateless and provides no uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `time` - Wall time to encode.
    /// * `sequence` - Sequence to encode within that timestamp unit.
    ///
    /// # Returns
    ///
    /// The composed numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] if `time` is before the configured
    /// epoch, [`IdError::GeneratorExpired`] if `time` has reached the exclusive
    /// expiration boundary, or [`IdError::SequenceOverflow`] when `sequence`
    /// does not fit the layout.
    #[cfg(feature = "qubit-snowflake")]
    #[inline(always)]
    pub(crate) fn compose_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<u64, IdError> {
        self.ensure_active(time)?;
        let observation = self.observation_for(time)?;
        self.layout.compose(observation.timestamp, sequence)
    }

    /// Returns the configured layout.
    ///
    /// # Returns
    ///
    /// The layout used to compose generated IDs.
    #[must_use]
    #[inline(always)]
    pub(crate) const fn layout(&self) -> &L {
        &self.layout
    }

    /// Returns the configured timestamp origin.
    ///
    /// # Returns
    ///
    /// The timestamp origin represented by timestamp zero.
    #[inline(always)]
    pub(crate) const fn epoch(&self) -> SystemTime {
        self.epoch
    }

    /// Returns the exclusive expiration boundary.
    ///
    /// # Returns
    ///
    /// The first wall time that cannot be represented by this core.
    #[inline(always)]
    pub(crate) const fn expires_at(&self) -> SystemTime {
        self.expires_at
    }

    /// Returns the maximum tolerated raw wall-clock rollback.
    ///
    /// # Returns
    ///
    /// The largest rollback duration accepted by the allocation state.
    #[cfg(feature = "qubit-snowflake")]
    #[inline(always)]
    pub(crate) const fn max_clock_skew(&self) -> Duration {
        self.max_clock_skew
    }

    /// Converts one wall time into a layout-aware clock observation.
    ///
    /// # Parameters
    ///
    /// * `time` - Wall time to measure from the configured epoch.
    ///
    /// # Returns
    ///
    /// The raw elapsed duration, encoded timestamp, and retry delay.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` precedes the
    /// configured epoch.
    fn observation_for(
        &self,
        time: SystemTime,
    ) -> Result<ClockObservation, IdError> {
        ClockObservation::from_time(
            time,
            self.epoch,
            self.layout.time_unit(),
            self.layout.max_timestamp(),
        )
    }

    /// Rejects wall times at or beyond the exclusive expiration boundary.
    ///
    /// # Parameters
    ///
    /// * `observed_at` - Wall time to compare with the cached boundary.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the generator remains active.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::GeneratorExpired`] when `observed_at` is equal to or
    /// later than the exclusive expiration boundary.
    fn ensure_active(&self, observed_at: SystemTime) -> Result<(), IdError> {
        if observed_at >= self.expires_at {
            return Err(IdError::GeneratorExpired {
                observed_at,
                expires_at: self.expires_at,
            });
        }
        Ok(())
    }
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Classic 41/10/12 Snowflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use parking_lot::Mutex;
use qubit_clock::{
    BlockingSleeper,
    Timer,
    WallClock,
};

use super::RestartPolicy;
use super::internal::{
    ClockObservation,
    GenerationState,
    block_until_generated,
};
use super::snowflake_generator_builder::SnowflakeGeneratorBuilder;
use super::snowflake_layout::SnowflakeLayout;
use crate::{
    GenerationOutcome,
    IdError,
    IdGenerator,
};

/// Classic Snowflake generator using 41 timestamp, 10 node, and 12 sequence
/// bits.
///
/// # Uniqueness
///
/// The generator is thread-safe. Successful [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] calls on one shared live instance never return
/// the same ID. A process should share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive node identifier when its epoch can produce IDs in the same
/// namespace.
///
/// The default [`RestartPolicy::Immediate`] allocates sequence zero in the
/// currently observed millisecond without waiting. Allocation state is not
/// persisted. State loss or replacement can repeat an ID only when the
/// instances use the same effective identity (`node_id`), layout, and
/// reference time (`epoch`), allocate in the same logical millisecond, and use
/// overlapping sequence ranges.
///
/// [`RestartPolicy::WaitNextSlice`] waits until after the first observed
/// millisecond. It reduces sequential-replacement risk only when that
/// millisecond is not earlier than the predecessor's last allocated
/// millisecond. Because predecessor state is not persisted, clock rollback
/// across a restart can still repeat IDs. The policy also does not coordinate
/// concurrent same-identity instances, which can cross the fence together and
/// allocate overlapping sequence ranges. Such deployments require external
/// exclusivity.
///
/// # Blocking and clock behavior
///
/// [`IdGenerator::try_next_id`] performs one attempt and never sleeps for clock
/// progress or invokes the configured sleeper, although it can briefly contend
/// for the internal mutex. [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] wait across retry outcomes. They may wait
/// indefinitely when the wall clock stalls or
/// the injected sleeper does not cause wall time to progress. A backwards
/// clock movement returns
/// [`IdError::ClockMovedBackwards`] immediately.
#[must_use]
pub struct SnowflakeGenerator {
    /// Bit layout and node identifier used to compose generated IDs.
    layout: SnowflakeLayout,
    /// Timestamp origin used by encoded timestamps.
    epoch: SystemTime,
    /// Exclusive timestamp expiration boundary.
    expires_at: SystemTime,
    /// Wall clock sampled by allocation attempts.
    wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    blocking_sleeper: BlockingSleeper,
    /// Synchronized raw-clock and sequence allocation state.
    state: Mutex<GenerationState>,
}

impl SnowflakeGenerator {
    /// Creates a classic Snowflake generator with the default Qubit epoch.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier in `0..=1023`.
    ///
    /// # Returns
    ///
    /// A configured generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] when `node_id` does not fit in 10
    /// bits.
    ///
    /// # Panics
    ///
    /// Panics when the current wall time is equal to or later than the
    /// layout's exclusive expiration boundary.
    #[inline(always)]
    pub fn new(node_id: u64) -> Result<Self, IdError> {
        Self::builder(node_id).build()
    }

    /// Creates a configurable builder for the specified node identifier.
    ///
    /// Node validation is performed when [`SnowflakeGeneratorBuilder::build`]
    /// is called.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable classic Snowflake generator builder.
    #[inline(always)]
    pub fn builder(node_id: u64) -> SnowflakeGeneratorBuilder {
        SnowflakeGeneratorBuilder::new(node_id)
    }

    /// Constructs a generator from a complete builder configuration.
    ///
    /// # Arguments
    ///
    /// * `layout` - Validated classic Snowflake layout.
    /// * `epoch` - Timestamp origin used by the generator.
    /// * `expires_at` - Exclusive timestamp expiration boundary.
    /// * `restart_policy` - Policy controlling the first allocation.
    /// * `wall_clock` - Wall clock sampled during allocation.
    /// * `timer` - Timer adapted for blocking generation.
    ///
    /// # Returns
    ///
    /// A generator containing the complete builder configuration.
    #[inline]
    pub(super) fn from_config(
        layout: SnowflakeLayout,
        epoch: SystemTime,
        expires_at: SystemTime,
        restart_policy: RestartPolicy,
        wall_clock: Arc<dyn WallClock>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            layout,
            epoch,
            expires_at,
            wall_clock,
            blocking_sleeper: BlockingSleeper::new(timer),
            state: Mutex::new(GenerationState::new(restart_policy)),
        }
    }

    /// Returns the layout used to compose generated IDs.
    ///
    /// # Returns
    ///
    /// Configured classic Snowflake layout.
    #[inline(always)]
    pub const fn layout(&self) -> &SnowflakeLayout {
        &self.layout
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    ///
    /// Timestamp origin.
    #[must_use]
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
        self.epoch
    }

    /// Returns the exclusive timestamp expiration boundary.
    ///
    /// The generator is expired when the wall clock is equal to or later than
    /// this time.
    ///
    /// # Returns
    ///
    /// Exclusive expiration boundary.
    #[must_use]
    #[inline(always)]
    pub const fn expires_at(&self) -> SystemTime {
        self.expires_at
    }

    /// Converts a clock time into a raw millisecond observation.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to convert.
    ///
    /// # Returns
    ///
    /// Raw elapsed time and encoded milliseconds since the epoch.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` is before the epoch.
    #[inline(always)]
    fn observation_for(
        &self,
        time: SystemTime,
    ) -> Result<ClockObservation, IdError> {
        ClockObservation::from_time(
            time,
            self.epoch,
            Duration::from_millis(1),
            self.layout.max_timestamp(),
        )
    }
}

impl IdGenerator for SnowflakeGenerator {
    type Id = u64;
    type Error = IdError;

    /// Performs one classic Snowflake allocation attempt without sleeping.
    ///
    /// # Returns
    ///
    /// A generated ID or the positive duration before another attempt.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] when the clock or encoded timestamp is invalid.
    fn try_next_id(&self) -> Result<GenerationOutcome<Self::Id>, Self::Error> {
        let outcome = {
            let mut state = self.state.lock();
            let observation = self.observation_for(self.wall_clock.now())?;
            state.reserve(
                observation,
                self.layout.max_sequence(),
                Duration::ZERO,
            )?
        };
        match outcome {
            GenerationOutcome::Generated(time_slice) => self
                .layout
                .compose(time_slice.timestamp, time_slice.sequence)
                .map(GenerationOutcome::Generated),
            GenerationOutcome::RetryAfter(duration) => {
                Ok(GenerationOutcome::RetryAfter(duration))
            }
        }
    }

    /// Generates the next classic Snowflake ID.
    ///
    /// This method blocks across retryable sequence exhaustion.
    ///
    /// # Returns
    ///
    /// The next generated classic Snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns an allocation error or [`IdError::SleepFailed`] when a retry
    /// delay cannot be completed.
    #[inline(always)]
    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        block_until_generated(&self.blocking_sleeper, || self.try_next_id())
    }

    /// Formats a classic Snowflake ID as unsigned decimal text.
    ///
    /// # Arguments
    ///
    /// * `id` - Classic Snowflake ID to format.
    ///
    /// # Returns
    ///
    /// Unsigned decimal text for `id`.
    #[inline(always)]
    fn format_id(&self, id: &Self::Id) -> String {
        id.to_string()
    }
}

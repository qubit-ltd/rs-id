// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Qubit snowflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use parking_lot::Mutex;
use qubit_clock::{
    BlockingSleeper,
    WallClock,
};

use super::QubitSnowflakeLayout;
use super::RestartPolicy;
use super::internal::{
    ClockObservation,
    GenerationState,
    block_until_generated,
};
use super::qubit_snowflake_generator_builder::QubitSnowflakeGeneratorBuilder;
use crate::{
    GenerationOutcome,
    IdError,
    IdGenerator,
};

/// Qubit Snowflake generator.
///
/// This generator uses the Qubit fixed-header layout, including mode and
/// precision bits. The default constructor uses sequential mode, second
/// precision, the caller-provided host, and epoch `2018-12-02T00:00:00Z`.
///
/// # Uniqueness
///
/// The generator is thread-safe. Successful [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] calls on one shared live instance never return
/// the same ID. A process should share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive host identifier when its layout and epoch can produce IDs in
/// the same namespace.
///
/// The default [`RestartPolicy::Immediate`] allocates sequence zero in the
/// currently observed time slice without waiting. Allocation state is not
/// persisted. State loss or replacement can repeat an ID only when the
/// instances use the same effective identity (`host`), layout (`mode` and
/// `precision`), and reference time (`epoch`), allocate in the same logical
/// time slice, and use overlapping sequence ranges.
///
/// [`RestartPolicy::WaitNextSlice`] waits until after the first observed time
/// slice. It reduces sequential-replacement risk only when that slice is not
/// earlier than the predecessor's last allocated slice. Because predecessor
/// state is not persisted, clock rollback across a restart can still repeat
/// IDs. The policy also does not coordinate concurrent same-identity instances,
/// which can cross the fence together and allocate overlapping sequence ranges.
/// Such deployments require external exclusivity.
///
/// # Blocking and clock behavior
///
/// [`IdGenerator::try_next_id`] performs one attempt and never sleeps for clock
/// progress or invokes the configured sleeper, although it can briefly contend
/// for the internal mutex. [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] wait across retry outcomes. They may wait
/// indefinitely when the wall clock stalls or
/// the injected sleeper does not cause wall time to progress. A backwards
/// clock movement within `max_clock_skew` is retried after waiting; a larger
/// movement returns
/// [`IdError::ClockMovedBackwards`].
pub struct QubitSnowflakeGenerator {
    /// Bit layout used to compose generated IDs.
    layout: QubitSnowflakeLayout,
    /// Timestamp origin used by encoded timestamps.
    epoch: SystemTime,
    /// Maximum tolerated raw wall-clock rollback.
    max_clock_skew: Duration,
    /// Wall clock sampled by allocation attempts.
    wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    blocking_sleeper: Arc<dyn BlockingSleeper>,
    /// Synchronized raw-clock and sequence allocation state.
    state: Mutex<GenerationState>,
}

impl QubitSnowflakeGenerator {
    /// Creates a generator with Qubit defaults.
    ///
    /// # Arguments
    ///
    /// * `host` - Host identifier in `0..=511`.
    ///
    /// # Returns
    ///
    /// A configured generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit in the host
    /// field.
    #[inline(always)]
    pub fn new(host: u64) -> Result<Self, IdError> {
        Self::builder(host).build()
    }

    /// Creates a configurable generator builder for the specified host.
    ///
    /// Host validation is performed when
    /// [`QubitSnowflakeGeneratorBuilder::build`] is called.
    ///
    /// # Arguments
    ///
    /// * `host` - Host identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable Qubit snowflake generator builder.
    #[must_use]
    #[inline(always)]
    pub fn builder(host: u64) -> QubitSnowflakeGeneratorBuilder {
        QubitSnowflakeGeneratorBuilder::new(host)
    }

    /// Constructs a generator from a validated builder configuration.
    ///
    /// # Arguments
    ///
    /// * `layout` - Validated Qubit bit layout.
    /// * `epoch` - Timestamp origin used by the generator.
    /// * `max_clock_skew` - Largest raw clock rollback that may be retried.
    /// * `restart_policy` - Policy controlling the first allocation.
    /// * `wall_clock` - Wall clock sampled during allocation.
    /// * `blocking_sleeper` - Sleeper used by blocking generation.
    ///
    /// # Returns
    ///
    /// A generator containing the complete builder configuration.
    #[inline]
    pub(super) fn from_config(
        layout: QubitSnowflakeLayout,
        epoch: SystemTime,
        max_clock_skew: Duration,
        restart_policy: RestartPolicy,
        wall_clock: Arc<dyn WallClock>,
        blocking_sleeper: Arc<dyn BlockingSleeper>,
    ) -> Self {
        Self {
            layout,
            epoch,
            max_clock_skew,
            wall_clock,
            blocking_sleeper,
            state: Mutex::new(GenerationState::new(restart_policy)),
        }
    }

    /// Returns the Qubit bit layout.
    ///
    /// # Returns
    ///
    /// Layout used to compose generated IDs.
    #[inline(always)]
    pub const fn layout(&self) -> &QubitSnowflakeLayout {
        &self.layout
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    ///
    /// Timestamp origin.
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
        self.epoch
    }

    /// Returns the maximum tolerated backwards clock movement.
    ///
    /// # Returns
    ///
    /// Maximum tolerated raw wall-clock rollback.
    #[inline(always)]
    pub const fn max_clock_skew(&self) -> Duration {
        self.max_clock_skew
    }

    /// Generates an ID for an explicit time and sequence.
    ///
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to encode.
    /// * `sequence` - Sequence value inside the encoded time slice.
    ///
    /// # Returns
    ///
    /// Encoded ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] if `time` is before the configured
    /// epoch. Returns [`IdError::TimestampOverflow`] or
    /// [`IdError::SequenceOverflow`] when the computed timestamp or provided
    /// sequence does not fit the layout.
    #[inline(always)]
    pub fn generate_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<u64, IdError> {
        let observation = self.observation_for(time)?;
        self.layout.compose(observation.timestamp, sequence)
    }

    /// Converts a time value into a raw and precision-aware observation.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to convert.
    ///
    /// # Returns
    ///
    /// Raw elapsed time and encoded timestamp in the configured precision.
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
            Duration::from_millis(self.layout.precision().divisor_millis()),
            self.layout.max_timestamp(),
        )
    }
}

impl IdGenerator for QubitSnowflakeGenerator {
    type Id = u64;
    type Error = IdError;

    /// Performs one Qubit snowflake allocation attempt without sleeping.
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
                self.max_clock_skew,
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

    /// Generates the next Qubit snowflake ID.
    ///
    /// Timestamp and sequence pairs are reserved while holding the generator
    /// mutex. When the current sequence range is exhausted, this method
    /// releases the mutex, waits for a later time slice, and then competes
    /// for a new reservation. The method can therefore block for
    /// approximately one time slice while the clock advances normally, or
    /// longer while tolerating a configured backwards clock skew.
    ///
    /// # Returns
    ///
    /// The next generated Qubit snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns an allocation error or [`IdError::SleepFailed`] when a retry
    /// delay cannot be completed.
    #[inline(always)]
    fn next_id(&self) -> Result<Self::Id, Self::Error> {
        block_until_generated(self.blocking_sleeper.as_ref(), || {
            self.try_next_id()
        })
    }

    /// Formats a Qubit snowflake ID as unsigned decimal text.
    ///
    /// # Arguments
    ///
    /// * `id` - Qubit snowflake ID to format.
    ///
    /// # Returns
    ///
    /// Unsigned decimal text for `id`.
    #[inline(always)]
    fn format_id(&self, id: &Self::Id) -> String {
        id.to_string()
    }
}

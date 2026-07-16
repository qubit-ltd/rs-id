// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Sonyflake-style 63-bit ID generator.

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

use super::internal::{
    ClockObservation,
    GenerationState,
    block_until_generated,
    panic_if_expired,
};
use super::sonyflake_generator_builder::SonyflakeGeneratorBuilder;
use super::sonyflake_layout::SonyflakeLayout;
use crate::{
    GenerationOutcome,
    IdError,
    IdGenerator,
};

pub(super) const DEFAULT_START_MILLIS: u64 = 1_735_689_600_000;

/// Sonyflake-style generator using configurable time, sequence, and machine
/// bits.
///
/// By default, the layout is compatible with Sonyflake's commonly documented
/// allocation: 39 bits of time in 10 ms units, 8 sequence bits, and 16 machine
/// bits. The sign bit is not used.
///
/// # Uniqueness
///
/// The generator is thread-safe. Successful [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] calls on one shared live instance never return
/// the same ID. A process should share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive machine identifier when its layout and start time can produce
/// IDs in the same namespace.
///
/// The default [`crate::RestartPolicy::Immediate`] allocates sequence zero in
/// the currently observed time unit without waiting. Allocation state is not
/// persisted. State loss or replacement can repeat an ID only when the
/// instances use the same effective identity (`machine_id`), layout (field
/// widths and `time_unit`), and reference time (`start_time`), allocate in the
/// same logical time unit, and use overlapping sequence ranges.
///
/// [`crate::RestartPolicy::WaitNextSlice`] waits until after the first observed
/// time unit. It reduces sequential-replacement risk only when that unit is not
/// earlier than the predecessor's last allocated unit. Because predecessor
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
/// the injected sleeper does not cause wall time to progress. IDs are never
/// allocated from a logical future time unit. A backwards clock movement
/// returns [`IdError::ClockMovedBackwards`] immediately.
#[must_use]
pub struct SonyflakeGenerator {
    /// Bit layout and machine identifier used to compose generated IDs.
    layout: SonyflakeLayout,
    /// Elapsed-time origin encoded by generated IDs.
    start_time: SystemTime,
    /// Exclusive elapsed-time expiration boundary.
    expires_at: SystemTime,
    /// Wall clock sampled by allocation attempts.
    wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    blocking_sleeper: Arc<dyn BlockingSleeper>,
    /// Synchronized raw-clock and sequence allocation state.
    state: Mutex<GenerationState>,
}

impl SonyflakeGenerator {
    /// Creates a Sonyflake-style generator with default layout and epoch.
    ///
    /// # Arguments
    ///
    /// * `machine_id` - Machine identifier in `0..=65535`.
    ///
    /// # Returns
    ///
    /// A configured generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::MachineIdOutOfRange`] when `machine_id` does not fit
    /// in the default 16-bit machine field.
    ///
    /// # Panics
    ///
    /// Panics when the current wall time is equal to or later than the
    /// layout's exclusive expiration boundary.
    #[inline(always)]
    pub fn new(machine_id: u64) -> Result<Self, IdError> {
        Self::builder(machine_id).build()
    }

    /// Creates a configurable generator builder for a machine identifier.
    ///
    /// Configuration validation is performed when
    /// [`SonyflakeGeneratorBuilder::build`] is called.
    ///
    /// # Arguments
    ///
    /// * `machine_id` - Machine identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable Sonyflake-style generator builder.
    #[inline(always)]
    pub fn builder(machine_id: u64) -> SonyflakeGeneratorBuilder {
        SonyflakeGeneratorBuilder::new(machine_id)
    }

    /// Validates builder fields and constructs a configured generator.
    ///
    /// # Arguments
    ///
    /// * `builder` - Complete Sonyflake-style builder configuration.
    ///
    /// # Returns
    ///
    /// A generator containing the validated builder configuration.
    ///
    /// # Errors
    ///
    /// Returns a layout, time-unit, start-time, machine-range, or expiration
    /// representation error when the configuration is invalid.
    ///
    /// # Panics
    ///
    /// Panics when the configured wall clock is equal to or later than the
    /// exclusive expiration boundary.
    pub(super) fn from_builder(
        builder: SonyflakeGeneratorBuilder,
    ) -> Result<Self, IdError> {
        let SonyflakeGeneratorBuilder {
            machine_id,
            bits_sequence,
            bits_machine: bits_machine_id,
            time_unit,
            start_time,
            restart_policy,
            wall_clock,
            blocking_sleeper,
        } = builder;
        let layout = SonyflakeLayout::new(
            machine_id,
            bits_sequence,
            bits_machine_id,
            time_unit,
        )?;
        let expires_at = layout.expires_at(start_time)?;

        let current_time = wall_clock.now();
        if start_time > current_time {
            return Err(IdError::StartTimeAhead {
                start_time,
                current_time,
            });
        }
        panic_if_expired("Sonyflake", current_time, expires_at);

        Ok(Self {
            layout,
            start_time,
            expires_at,
            wall_clock,
            blocking_sleeper,
            state: Mutex::new(GenerationState::new(restart_policy)),
        })
    }

    /// Returns the layout used to compose generated IDs.
    ///
    /// # Returns
    ///
    /// Configured Sonyflake layout.
    #[inline(always)]
    pub const fn layout(&self) -> &SonyflakeLayout {
        &self.layout
    }

    /// Returns the configured start time.
    ///
    /// # Returns
    ///
    /// Elapsed-time origin used by this generator.
    #[must_use]
    #[inline(always)]
    pub const fn start_time(&self) -> SystemTime {
        self.start_time
    }

    /// Returns the exclusive elapsed-time expiration boundary.
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

    /// Converts a time value into a raw Sonyflake clock observation.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to convert.
    ///
    /// # Returns
    ///
    /// Raw elapsed time and encoded units since the start time.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` is before `start_time`.
    #[inline(always)]
    fn observation_for(
        &self,
        time: SystemTime,
    ) -> Result<ClockObservation, IdError> {
        ClockObservation::from_time(
            time,
            self.start_time,
            self.layout.time_unit(),
            self.layout.max_elapsed_time(),
        )
    }
}

impl IdGenerator for SonyflakeGenerator {
    type Id = u64;
    type Error = IdError;

    /// Performs one Sonyflake allocation attempt without sleeping.
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

    /// Generates the next Sonyflake-style ID.
    ///
    /// This method blocks across retryable sequence exhaustion.
    ///
    /// # Returns
    ///
    /// The next generated Sonyflake-style ID.
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

    /// Formats a Sonyflake ID as unsigned decimal text.
    ///
    /// # Arguments
    ///
    /// * `id` - Sonyflake-style ID to format.
    ///
    /// # Returns
    ///
    /// Unsigned decimal text for `id`.
    #[inline(always)]
    fn format_id(&self, id: &Self::Id) -> String {
        id.to_string()
    }
}

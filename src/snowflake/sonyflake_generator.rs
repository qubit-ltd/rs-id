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
};
use super::sonyflake_generator_builder::SonyflakeGeneratorBuilder;
use crate::{
    GenerationOutcome,
    IdError,
    IdGenerator,
};

pub(super) const DEFAULT_BITS_SEQUENCE: u8 = 8;
pub(super) const DEFAULT_BITS_MACHINE: u8 = 16;
pub(super) const DEFAULT_TIME_UNIT_NANOS: u128 = 10_000_000;
const MIN_TIME_UNIT_NANOS: u128 = 1_000_000;
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
pub struct SonyflakeGenerator {
    /// Width of the elapsed-time field.
    bits_time: u8,
    /// Width of the sequence field.
    bits_sequence: u8,
    /// Width of the machine field.
    bits_machine: u8,
    /// Duration represented by one elapsed-time unit.
    time_unit: Duration,
    /// Elapsed-time origin encoded by generated IDs.
    start_time: SystemTime,
    /// Machine identifier encoded in generated IDs.
    machine_id: u64,
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
    #[must_use]
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
    /// Returns a layout, time-unit, start-time, or machine-range error when
    /// the configuration is invalid.
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
        let bits_sequence = Self::normalize_bits(
            "sequence",
            bits_sequence,
            DEFAULT_BITS_SEQUENCE,
        )?;
        let bits_machine = Self::normalize_bits(
            "machine",
            bits_machine_id,
            DEFAULT_BITS_MACHINE,
        )?;
        let bits_time = 63_u8
            .checked_sub(bits_sequence)
            .and_then(|value| value.checked_sub(bits_machine))
            .ok_or(IdError::InvalidBitLength {
                name: "time",
                bits: 0,
                reason: "63 - sequence bits - machine bits must be at least 32",
            })?;
        if bits_time < 32 {
            return Err(IdError::InvalidBitLength {
                name: "time",
                bits: bits_time,
                reason: "time bit length must be at least 32",
            });
        }

        let nanos = time_unit.as_nanos();
        if nanos < MIN_TIME_UNIT_NANOS {
            return Err(IdError::InvalidTimeUnit {
                nanos,
                min_nanos: MIN_TIME_UNIT_NANOS,
            });
        }

        let current_time = wall_clock.now();
        if start_time > current_time {
            return Err(IdError::StartTimeAhead {
                start_time,
                current_time,
            });
        }

        let max_machine_id = (1_u64 << bits_machine) - 1;
        if machine_id > max_machine_id {
            return Err(IdError::MachineIdOutOfRange {
                machine_id,
                max: max_machine_id,
            });
        }

        Ok(Self {
            bits_time,
            bits_sequence,
            bits_machine,
            time_unit,
            start_time,
            machine_id,
            wall_clock,
            blocking_sleeper,
            state: Mutex::new(GenerationState::new(restart_policy)),
        })
    }

    /// Returns the number of time bits.
    ///
    /// # Returns
    ///
    /// Time bit length.
    #[inline(always)]
    pub const fn bits_time(&self) -> u8 {
        self.bits_time
    }

    /// Returns the number of sequence bits.
    ///
    /// # Returns
    ///
    /// Sequence bit length.
    #[inline(always)]
    pub const fn bits_sequence(&self) -> u8 {
        self.bits_sequence
    }

    /// Returns the number of machine bits.
    ///
    /// # Returns
    ///
    /// Machine bit length.
    #[inline(always)]
    pub const fn bits_machine(&self) -> u8 {
        self.bits_machine
    }

    /// Returns the configured time unit.
    ///
    /// # Returns
    ///
    /// Duration represented by one elapsed-time unit.
    #[inline(always)]
    pub const fn time_unit(&self) -> Duration {
        self.time_unit
    }

    /// Returns the configured start time.
    ///
    /// # Returns
    ///
    /// Elapsed-time origin used by this generator.
    #[inline(always)]
    pub const fn start_time(&self) -> SystemTime {
        self.start_time
    }

    /// Returns the configured machine identifier.
    ///
    /// # Returns
    ///
    /// Machine identifier encoded in generated IDs.
    #[inline(always)]
    pub const fn machine_id(&self) -> u64 {
        self.machine_id
    }

    /// Returns the maximum representable elapsed time unit.
    ///
    /// # Returns
    ///
    /// Maximum elapsed time value.
    #[inline(always)]
    pub const fn max_elapsed_time(&self) -> u64 {
        (1_u64 << self.bits_time) - 1
    }

    /// Returns the maximum representable sequence.
    ///
    /// # Returns
    ///
    /// Maximum sequence number.
    #[inline(always)]
    pub const fn max_sequence(&self) -> u64 {
        (1_u64 << self.bits_sequence) - 1
    }

    /// Returns the maximum representable machine identifier.
    ///
    /// # Returns
    ///
    /// Maximum machine identifier.
    #[inline(always)]
    pub const fn max_machine_id(&self) -> u64 {
        (1_u64 << self.bits_machine) - 1
    }

    /// Composes a Sonyflake-style ID from explicit parts.
    ///
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
    ///
    /// # Arguments
    ///
    /// * `elapsed_time` - Time units elapsed since the start time.
    /// * `sequence` - Sequence value inside the time unit.
    /// * `machine_id` - Machine identifier.
    ///
    /// # Returns
    ///
    /// Encoded ID.
    ///
    /// # Errors
    ///
    /// Returns range errors when any part does not fit the configured layout.
    pub fn compose(
        &self,
        elapsed_time: u64,
        sequence: u64,
        machine_id: u64,
    ) -> Result<u64, IdError> {
        if elapsed_time > self.max_elapsed_time() {
            return Err(IdError::TimestampOverflow {
                timestamp: elapsed_time,
                max: self.max_elapsed_time(),
            });
        }
        if sequence > self.max_sequence() {
            return Err(IdError::SequenceOverflow {
                sequence,
                max: self.max_sequence(),
            });
        }
        if machine_id > self.max_machine_id() {
            return Err(IdError::MachineIdOutOfRange {
                machine_id,
                max: self.max_machine_id(),
            });
        }
        Ok((elapsed_time << (self.bits_sequence + self.bits_machine))
            | (sequence << self.bits_machine)
            | machine_id)
    }

    /// Extracts elapsed time from a Sonyflake-style ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID generated by this layout.
    ///
    /// # Returns
    ///
    /// Elapsed time units since the start time.
    #[inline(always)]
    pub fn extract_elapsed_time(&self, id: u64) -> u64 {
        id >> (self.bits_sequence + self.bits_machine)
    }

    /// Extracts sequence from a Sonyflake-style ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID generated by this layout.
    ///
    /// # Returns
    ///
    /// Sequence number.
    #[inline(always)]
    pub fn extract_sequence(&self, id: u64) -> u64 {
        let mask = ((1_u64 << self.bits_sequence) - 1) << self.bits_machine;
        (id & mask) >> self.bits_machine
    }

    /// Extracts machine ID from a Sonyflake-style ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID generated by this layout.
    ///
    /// # Returns
    ///
    /// Machine identifier.
    #[inline(always)]
    pub fn extract_machine_id(&self, id: u64) -> u64 {
        id & ((1_u64 << self.bits_machine) - 1)
    }

    /// Normalizes and validates a Sonyflake bit length.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the setting for diagnostics.
    /// * `bits` - Provided bit length.
    /// * `default_bits` - Default bit length used when `bits` is zero.
    ///
    /// # Returns
    ///
    /// Normalized bit length.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::InvalidBitLength`] when the normalized value is 31 or
    /// greater.
    #[inline]
    fn normalize_bits(
        name: &'static str,
        bits: u8,
        default_bits: u8,
    ) -> Result<u8, IdError> {
        let normalized = if bits == 0 { default_bits } else { bits };
        if normalized >= 31 {
            return Err(IdError::InvalidBitLength {
                name,
                bits: normalized,
                reason: "bit length must be less than 31",
            });
        }
        Ok(normalized)
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
    fn observation_for(
        &self,
        time: SystemTime,
    ) -> Result<ClockObservation, IdError> {
        ClockObservation::from_time(
            time,
            self.start_time,
            self.time_unit,
            self.max_elapsed_time(),
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
            state.reserve(observation, self.max_sequence(), Duration::ZERO)?
        };
        match outcome {
            GenerationOutcome::Generated(time_slice) => self
                .compose(
                    time_slice.timestamp,
                    time_slice.sequence,
                    self.machine_id,
                )
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

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for the Sonyflake-style generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_clock::{
    BlockingSleeper,
    WallClock,
};

use super::RestartPolicy;
use super::internal::{
    default_blocking_sleeper,
    default_wall_clock,
};
use super::sonyflake_generator::{
    DEFAULT_BITS_MACHINE,
    DEFAULT_BITS_SEQUENCE,
    DEFAULT_START_MILLIS,
    DEFAULT_TIME_UNIT_NANOS,
    SonyflakeGenerator,
};
use crate::IdError;

/// Configures and constructs a [`SonyflakeGenerator`].
///
/// The required machine identifier is supplied when the builder is created.
/// Unspecified options use the Sonyflake-style defaults: 8 sequence bits, 16
/// machine bits, a 10 ms time unit, start time `2025-01-01T00:00:00Z`, and the
/// [`RestartPolicy::Immediate`] policy with standard clock and sleeper
/// capabilities.
pub struct SonyflakeGeneratorBuilder {
    /// Machine identifier encoded in generated IDs.
    pub(super) machine_id: u64,
    /// Requested sequence field width.
    pub(super) bits_sequence: u8,
    /// Requested machine field width.
    pub(super) bits_machine: u8,
    /// Duration represented by one elapsed-time unit.
    pub(super) time_unit: Duration,
    /// Elapsed-time origin encoded by generated IDs.
    pub(super) start_time: SystemTime,
    /// First-allocation policy.
    pub(super) restart_policy: RestartPolicy,
    /// Wall clock sampled during validation and allocation.
    pub(super) wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    pub(super) blocking_sleeper: Arc<dyn BlockingSleeper>,
}

impl SonyflakeGeneratorBuilder {
    /// Creates a builder for the specified machine identifier.
    ///
    /// Machine and layout validation is deferred until [`Self::build`].
    ///
    /// # Arguments
    ///
    /// * `machine_id` - Machine identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A builder using the Sonyflake-style defaults and standard clocks.
    pub(crate) fn new(machine_id: u64) -> Self {
        Self {
            machine_id,
            bits_sequence: DEFAULT_BITS_SEQUENCE,
            bits_machine: DEFAULT_BITS_MACHINE,
            time_unit: Duration::from_nanos(DEFAULT_TIME_UNIT_NANOS as u64),
            start_time: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_START_MILLIS),
            restart_policy: RestartPolicy::Immediate,
            wall_clock: default_wall_clock(),
            blocking_sleeper: default_blocking_sleeper(),
        }
    }

    /// Sets the sequence field width.
    ///
    /// A value of zero selects the default width of 8 bits.
    ///
    /// # Arguments
    ///
    /// * `bits_sequence` - Sequence field width, or zero for the default.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn bits_sequence(mut self, bits_sequence: u8) -> Self {
        self.bits_sequence = bits_sequence;
        self
    }

    /// Sets the machine field width.
    ///
    /// A value of zero selects the default width of 16 bits.
    ///
    /// # Arguments
    ///
    /// * `bits_machine` - Machine field width, or zero for the default.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn bits_machine(mut self, bits_machine: u8) -> Self {
        self.bits_machine = bits_machine;
        self
    }

    /// Sets the duration represented by one encoded time unit.
    ///
    /// # Arguments
    ///
    /// * `time_unit` - Duration represented by one elapsed-time unit.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn time_unit(mut self, time_unit: Duration) -> Self {
        self.time_unit = time_unit;
        self
    }

    /// Sets the elapsed-time origin encoded by the generator.
    ///
    /// # Arguments
    ///
    /// * `start_time` - Elapsed-time origin to configure.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn start_time(mut self, start_time: SystemTime) -> Self {
        self.start_time = start_time;
        self
    }

    /// Sets the first-allocation behavior used after construction.
    ///
    /// # Arguments
    ///
    /// * `restart_policy` - Policy controlling the first allocation.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
        self.restart_policy = restart_policy;
        self
    }

    /// Sets the wall clock sampled during validation and allocation.
    ///
    /// # Arguments
    ///
    /// * `wall_clock` - Shared wall clock to sample.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn wall_clock(mut self, wall_clock: Arc<dyn WallClock>) -> Self {
        self.wall_clock = wall_clock;
        self
    }

    /// Sets the blocking sleeper used by [`crate::IdGenerator::next_id`].
    ///
    /// # Arguments
    ///
    /// * `blocking_sleeper` - Shared sleeper used for retry delays.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn blocking_sleeper(
        mut self,
        blocking_sleeper: Arc<dyn BlockingSleeper>,
    ) -> Self {
        self.blocking_sleeper = blocking_sleeper;
        self
    }

    /// Validates the complete configuration and constructs a generator.
    ///
    /// # Returns
    ///
    /// A configured Sonyflake-style generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::InvalidBitLength`] for an invalid allocation,
    /// [`IdError::InvalidTimeUnit`] for a sub-millisecond unit,
    /// [`IdError::StartTimeAhead`] when the start time is ahead of the clock,
    /// or [`IdError::MachineIdOutOfRange`] when the machine identifier does not
    /// fit its field.
    #[inline(always)]
    pub fn build(self) -> Result<SonyflakeGenerator, IdError> {
        SonyflakeGenerator::from_builder(self)
    }
}

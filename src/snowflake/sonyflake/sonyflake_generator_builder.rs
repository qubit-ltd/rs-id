// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for synchronous and asynchronous Sonyflake generators.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use qubit_clock::{
    Timer,
    WallClock,
};

use super::super::RestartPolicy;
use super::super::internal::{
    SnowflakeCore,
    default_timer,
    default_wall_clock,
    validate_generator_epoch,
    validate_generator_lifetime,
};
use super::sonyflake_generator::DEFAULT_EPOCH_MILLIS;
use super::sonyflake_layout::{
    DEFAULT_BITS_MACHINE,
    DEFAULT_BITS_SEQUENCE,
    DEFAULT_TIME_UNIT_NANOS,
};
use super::{
    SonyflakeGenerator,
    SonyflakeLayout,
};
use crate::IdGenerationError;

/// Configures synchronous or asynchronous Sonyflake-style generators.
#[must_use = "builders do nothing unless built"]
pub struct SonyflakeGeneratorBuilder {
    /// Machine identifier encoded in generated IDs.
    machine_id: u64,
    /// Requested sequence field width.
    bits_sequence: u8,
    /// Requested machine field width.
    bits_machine: u8,
    /// Duration represented by one elapsed-time unit.
    time_unit: Duration,
    /// Timestamp origin encoded by generated IDs.
    epoch: SystemTime,
    /// Maximum tolerated raw wall-clock rollback.
    max_clock_skew: Duration,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled during validation and allocation.
    wall_clock: Arc<dyn WallClock>,
    /// Timer used by blocking or asynchronous waits.
    timer: Arc<dyn Timer>,
}

impl SonyflakeGeneratorBuilder {
    /// Creates a builder for a machine identifier.
    ///
    /// # Parameters
    ///
    /// * `machine_id` - Machine identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A builder initialized with the default layout, zero clock-skew
    /// tolerance, clocks, and immediate restart policy.
    #[inline]
    pub(crate) fn new(machine_id: u64) -> Self {
        Self {
            machine_id,
            bits_sequence: DEFAULT_BITS_SEQUENCE,
            bits_machine: DEFAULT_BITS_MACHINE,
            time_unit: Duration::from_nanos(DEFAULT_TIME_UNIT_NANOS as u64),
            epoch: UNIX_EPOCH + Duration::from_millis(DEFAULT_EPOCH_MILLIS),
            max_clock_skew: Duration::ZERO,
            restart_policy: RestartPolicy::Immediate,
            wall_clock: default_wall_clock(),
            timer: default_timer(),
        }
    }

    /// Sets the sequence field width, using the default when zero.
    ///
    /// # Parameters
    ///
    /// * `bits_sequence` - Sequence field width, or zero for the default.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn bits_sequence(mut self, bits_sequence: u8) -> Self {
        self.bits_sequence = bits_sequence;
        self
    }

    /// Sets the machine field width, using the default when zero.
    ///
    /// # Parameters
    ///
    /// * `bits_machine` - Machine field width, or zero for the default.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn bits_machine(mut self, bits_machine: u8) -> Self {
        self.bits_machine = bits_machine;
        self
    }

    /// Sets the duration represented by one encoded time unit.
    ///
    /// # Parameters
    ///
    /// * `time_unit` - Duration represented by one elapsed-time unit.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn time_unit(mut self, time_unit: Duration) -> Self {
        self.time_unit = time_unit;
        self
    }

    /// Sets the timestamp origin encoded by generated IDs.
    ///
    /// # Parameters
    ///
    /// * `epoch` - Wall time represented by elapsed time zero.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn epoch(mut self, epoch: SystemTime) -> Self {
        self.epoch = epoch;
        self
    }

    /// Sets the maximum tolerated raw wall-clock rollback.
    ///
    /// # Parameters
    ///
    /// * `max_clock_skew` - Largest raw rollback that may be retried.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn max_clock_skew(mut self, max_clock_skew: Duration) -> Self {
        self.max_clock_skew = max_clock_skew;
        self
    }

    /// Sets the first-allocation behavior used after construction.
    ///
    /// # Parameters
    ///
    /// * `restart_policy` - Policy applied to the first allocation attempt.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
        self.restart_policy = restart_policy;
        self
    }

    /// Sets the wall clock sampled during validation and allocation.
    ///
    /// # Parameters
    ///
    /// * `wall_clock` - Wall clock sampled by validation and allocation.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn wall_clock(mut self, wall_clock: Arc<dyn WallClock>) -> Self {
        self.wall_clock = wall_clock;
        self
    }

    /// Sets the timer used by synchronous or asynchronous retry waits.
    ///
    /// Async generators may poll timer futures from a different runtime or
    /// execution context. A Tokio timer retains its target runtime handle, and
    /// that runtime must remain alive and driven. Synchronous generators block
    /// on the timer, so its backend must progress independently of the caller
    /// thread; do not rely on a Tokio current-thread runtime driven only by
    /// that same thread.
    ///
    /// # Parameters
    ///
    /// * `timer` - Timer adapted by synchronous and asynchronous generators.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn timer(mut self, timer: Arc<dyn Timer>) -> Self {
        self.timer = timer;
        self
    }

    /// Validates the configuration and constructs a generator.
    ///
    /// # Returns
    ///
    /// A synchronous Sonyflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::InvalidBitLength`],
    /// [`IdGenerationError::InvalidTimeUnit`], or
    /// [`IdGenerationError::MachineIdOutOfRange`] for an invalid layout,
    /// [`IdGenerationError::ExpirationTimeOverflow`] when the lifetime boundary
    /// cannot be represented, [`IdGenerationError::EpochAhead`] when
    /// the epoch is later than the configured wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when that clock has reached
    /// the boundary.
    #[inline]
    pub fn build(self) -> Result<SonyflakeGenerator, IdGenerationError> {
        let (core, timer) = self.into_core()?;
        Ok(SonyflakeGenerator::from_core(core, timer))
    }

    /// Converts the builder into a validated shared core and timer.
    ///
    /// # Returns
    ///
    /// The validated allocation core and configured timer.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::InvalidBitLength`],
    /// [`IdGenerationError::InvalidTimeUnit`], or
    /// [`IdGenerationError::MachineIdOutOfRange`] for an invalid layout,
    /// [`IdGenerationError::ExpirationTimeOverflow`] when the lifetime boundary
    /// cannot be represented, [`IdGenerationError::EpochAhead`] when
    /// the epoch is later than the configured wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when that clock has reached
    /// the boundary.
    fn into_core(
        self,
    ) -> Result<
        (SnowflakeCore<SonyflakeLayout>, Arc<dyn Timer>),
        IdGenerationError,
    > {
        let layout = SonyflakeLayout::new(
            self.machine_id,
            self.bits_sequence,
            self.bits_machine,
            self.time_unit,
        )?;
        let current_time = self.wall_clock.now();
        validate_generator_epoch(self.epoch, current_time)?;
        let expires_at = layout.expires_at(self.epoch)?;
        validate_generator_lifetime(self.epoch, expires_at, current_time)?;
        let core = SnowflakeCore::new(
            layout,
            self.epoch,
            expires_at,
            self.max_clock_skew,
            self.restart_policy,
            self.wall_clock,
        );
        Ok((core, self.timer))
    }
}

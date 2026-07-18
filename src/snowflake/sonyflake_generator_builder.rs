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

use super::internal::{
    SnowflakeCore,
    default_timer,
    default_wall_clock,
    panic_if_expired,
};
use super::sonyflake_generator::DEFAULT_START_MILLIS;
use super::sonyflake_layout::{
    DEFAULT_BITS_MACHINE,
    DEFAULT_BITS_SEQUENCE,
    DEFAULT_TIME_UNIT_NANOS,
};
use super::{
    AsyncSonyflakeGenerator,
    RestartPolicy,
    SonyflakeGenerator,
    SonyflakeLayout,
};
use crate::IdError;

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
    /// Elapsed-time origin encoded by generated IDs.
    start_time: SystemTime,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled during validation and allocation.
    wall_clock: Arc<dyn WallClock>,
    /// Timer used by blocking or asynchronous waits.
    timer: Arc<dyn Timer>,
}

impl SonyflakeGeneratorBuilder {
    /// Creates a builder for a machine identifier.
    #[inline]
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
            timer: default_timer(),
        }
    }

    /// Sets the sequence field width, using the default when zero.
    #[inline(always)]
    pub fn bits_sequence(mut self, bits_sequence: u8) -> Self {
        self.bits_sequence = bits_sequence;
        self
    }

    /// Sets the machine field width, using the default when zero.
    #[inline(always)]
    pub fn bits_machine(mut self, bits_machine: u8) -> Self {
        self.bits_machine = bits_machine;
        self
    }

    /// Sets the duration represented by one encoded time unit.
    #[inline(always)]
    pub fn time_unit(mut self, time_unit: Duration) -> Self {
        self.time_unit = time_unit;
        self
    }

    /// Sets the elapsed-time origin encoded by generated IDs.
    #[inline(always)]
    pub fn start_time(mut self, start_time: SystemTime) -> Self {
        self.start_time = start_time;
        self
    }

    /// Sets the first-allocation behavior used after construction.
    #[inline(always)]
    pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
        self.restart_policy = restart_policy;
        self
    }

    /// Sets the wall clock sampled during validation and allocation.
    #[inline(always)]
    pub fn wall_clock(mut self, wall_clock: Arc<dyn WallClock>) -> Self {
        self.wall_clock = wall_clock;
        self
    }

    /// Sets the timer used by synchronous or asynchronous retry waits.
    #[inline(always)]
    pub fn timer(mut self, timer: Arc<dyn Timer>) -> Self {
        self.timer = timer;
        self
    }

    /// Validates the configuration and constructs a synchronous generator.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] when a layout, start-time, or lifetime setting is
    /// invalid.
    ///
    /// # Panics
    ///
    /// Panics when the configured wall clock has reached the expiration
    /// boundary.
    #[inline]
    pub fn build(self) -> Result<SonyflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(SonyflakeGenerator::from_core(core, timer))
    }

    /// Validates the configuration and constructs an asynchronous generator.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] when a layout, start-time, or lifetime setting is
    /// invalid.
    ///
    /// # Panics
    ///
    /// Panics when the configured wall clock has reached the expiration
    /// boundary.
    #[inline]
    pub fn build_async(self) -> Result<AsyncSonyflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(AsyncSonyflakeGenerator::from_core(core, timer))
    }

    /// Converts the builder into a validated shared core and timer.
    fn into_core(
        self,
    ) -> Result<(SnowflakeCore<SonyflakeLayout>, Arc<dyn Timer>), IdError> {
        let layout = SonyflakeLayout::new(
            self.machine_id,
            self.bits_sequence,
            self.bits_machine,
            self.time_unit,
        )?;
        let expires_at = layout.expires_at(self.start_time)?;
        let current_time = self.wall_clock.now();
        if self.start_time > current_time {
            return Err(IdError::StartTimeAhead {
                start_time: self.start_time,
                current_time,
            });
        }
        panic_if_expired("Sonyflake", current_time, expires_at);
        let core = SnowflakeCore::new(
            layout,
            self.start_time,
            expires_at,
            Duration::ZERO,
            self.restart_policy,
            self.wall_clock,
        );
        Ok((core, self.timer))
    }
}

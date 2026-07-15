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
/// system wall clock.
pub struct SonyflakeGeneratorBuilder {
    machine_id: u64,
    bits_sequence: u8,
    bits_machine: u8,
    time_unit: Duration,
    start_time: SystemTime,
    clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
}

impl SonyflakeGeneratorBuilder {
    /// Creates a builder for the specified machine identifier.
    ///
    /// Machine and layout validation is deferred until [`Self::build`].
    pub(crate) fn new(machine_id: u64) -> Self {
        Self {
            machine_id,
            bits_sequence: DEFAULT_BITS_SEQUENCE,
            bits_machine: DEFAULT_BITS_MACHINE,
            time_unit: Duration::from_nanos(DEFAULT_TIME_UNIT_NANOS as u64),
            start_time: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_START_MILLIS),
            clock: Arc::new(SystemTime::now),
        }
    }

    /// Sets the sequence field width.
    ///
    /// A value of zero selects the default width of 8 bits.
    #[must_use]
    pub fn bits_sequence(mut self, bits_sequence: u8) -> Self {
        self.bits_sequence = bits_sequence;
        self
    }

    /// Sets the machine field width.
    ///
    /// A value of zero selects the default width of 16 bits.
    #[must_use]
    pub fn bits_machine(mut self, bits_machine: u8) -> Self {
        self.bits_machine = bits_machine;
        self
    }

    /// Sets the duration represented by one encoded time unit.
    #[must_use]
    pub fn time_unit(mut self, time_unit: Duration) -> Self {
        self.time_unit = time_unit;
        self
    }

    /// Sets the elapsed-time origin encoded by the generator.
    #[must_use]
    pub fn start_time(mut self, start_time: SystemTime) -> Self {
        self.start_time = start_time;
        self
    }

    /// Sets the wall-clock function used by the generator.
    ///
    /// The clock is sampled during [`Self::build`] to validate the start time
    /// and may later be called concurrently by multiple generator clients.
    #[must_use]
    pub fn clock<F>(mut self, clock: F) -> Self
    where
        F: Fn() -> SystemTime + Send + Sync + 'static,
    {
        self.clock = Arc::new(clock);
        self
    }

    /// Validates the complete configuration and constructs a generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::InvalidBitLength`] for an invalid allocation,
    /// [`IdError::InvalidTimeUnit`] for a sub-millisecond unit,
    /// [`IdError::StartTimeAhead`] when the start time is ahead of the clock,
    /// or [`IdError::MachineIdOutOfRange`] when the machine identifier does not
    /// fit its field.
    pub fn build(self) -> Result<SonyflakeGenerator, IdError> {
        SonyflakeGenerator::from_config(
            self.machine_id,
            self.bits_sequence,
            self.bits_machine,
            self.time_unit,
            self.start_time,
            self.clock,
        )
    }
}

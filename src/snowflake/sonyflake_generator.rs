// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Sonyflake-style 63-bit ID generator.

use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    SystemTime,
};

use parking_lot::Mutex;

use super::sonyflake_generator_builder::SonyflakeGeneratorBuilder;
use super::time_slice::TimeSlice;
use super::time_slice_reservation::{
    TimeSliceReservation,
    reserve_next,
};
use crate::{
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
/// The generator is thread-safe. Successful [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] calls on one shared live instance never return
/// the same ID. A process should share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive machine identifier when its layout and start time can produce
/// IDs in the same namespace.
///
/// The first generation call allocates sequence zero in the currently observed
/// time unit without waiting. Allocation state is not persisted, so replacing
/// an instance with the same machine, layout, and start time inside one time
/// unit can repeat IDs. Applications that reuse machines across restarts must
/// coordinate machine leases, wait outside the generator, or persist
/// allocation state.
///
/// # Blocking and clock behavior
/// A call after sequence exhaustion can block for approximately one configured
/// time unit while the clock advances normally. A stalled clock can block
/// indefinitely. IDs are never allocated from a
/// logical future time unit. A backwards clock movement returns
/// [`IdError::ClockMovedBackwards`] immediately.
pub struct SonyflakeGenerator {
    bits_time: u8,
    bits_sequence: u8,
    bits_machine: u8,
    time_unit: Duration,
    start_time: SystemTime,
    machine_id: u64,
    clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    state: Mutex<Option<TimeSlice>>,
}

impl SonyflakeGenerator {
    /// Creates a Sonyflake-style generator with default layout and epoch.
    ///
    /// # Parameters
    /// - `machine_id`: Machine identifier in `0..=65535`.
    ///
    /// # Returns
    /// A configured generator.
    ///
    /// # Errors
    /// Returns [`IdError::MachineIdOutOfRange`] when `machine_id` does not fit
    /// in the default 16-bit machine field.
    pub fn new(machine_id: u64) -> Result<Self, IdError> {
        Self::builder(machine_id).build()
    }

    /// Creates a configurable generator builder for a machine identifier.
    ///
    /// Configuration validation is performed when
    /// [`SonyflakeGeneratorBuilder::build`] is called.
    #[must_use]
    pub fn builder(machine_id: u64) -> SonyflakeGeneratorBuilder {
        SonyflakeGeneratorBuilder::new(machine_id)
    }

    /// Validates builder fields and constructs a configured generator.
    pub(super) fn from_config(
        machine_id: u64,
        bits_sequence: u8,
        bits_machine_id: u8,
        time_unit: Duration,
        start_time: SystemTime,
        clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    ) -> Result<Self, IdError> {
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

        if start_time > (clock)() {
            return Err(IdError::StartTimeAhead);
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
            clock,
            state: Mutex::new(None),
        })
    }

    /// Normalizes and validates a Sonyflake bit length.
    ///
    /// # Parameters
    /// - `name`: Name of the setting for diagnostics.
    /// - `bits`: Provided bit length.
    /// - `default_bits`: Default bit length used when `bits` is zero.
    ///
    /// # Returns
    /// Normalized bit length.
    ///
    /// # Errors
    /// Returns [`IdError::InvalidBitLength`] when the normalized value is 31 or
    /// greater.
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

    /// Returns the number of time bits.
    ///
    /// # Returns
    /// Time bit length.
    pub const fn bits_time(&self) -> u8 {
        self.bits_time
    }

    /// Returns the number of sequence bits.
    ///
    /// # Returns
    /// Sequence bit length.
    pub const fn bits_sequence(&self) -> u8 {
        self.bits_sequence
    }

    /// Returns the number of machine bits.
    ///
    /// # Returns
    /// Machine bit length.
    pub const fn bits_machine(&self) -> u8 {
        self.bits_machine
    }

    /// Returns the configured time unit.
    ///
    /// # Returns
    /// Duration represented by one elapsed-time unit.
    pub const fn time_unit(&self) -> Duration {
        self.time_unit
    }

    /// Returns the configured start time.
    ///
    /// # Returns
    /// Elapsed-time origin used by this generator.
    pub const fn start_time(&self) -> SystemTime {
        self.start_time
    }

    /// Returns the configured machine identifier.
    ///
    /// # Returns
    /// Machine identifier encoded in generated IDs.
    pub const fn machine_id(&self) -> u64 {
        self.machine_id
    }

    /// Returns the maximum representable elapsed time unit.
    ///
    /// # Returns
    /// Maximum elapsed time value.
    pub const fn max_elapsed_time(&self) -> u64 {
        (1_u64 << self.bits_time) - 1
    }

    /// Returns the maximum representable sequence.
    ///
    /// # Returns
    /// Maximum sequence number.
    pub const fn max_sequence(&self) -> u64 {
        (1_u64 << self.bits_sequence) - 1
    }

    /// Returns the maximum representable machine identifier.
    ///
    /// # Returns
    /// Maximum machine identifier.
    pub const fn max_machine_id(&self) -> u64 {
        (1_u64 << self.bits_machine) - 1
    }

    /// Composes a Sonyflake-style ID from explicit parts.
    ///
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
    ///
    /// # Parameters
    /// - `elapsed_time`: Time units elapsed since the start time.
    /// - `sequence`: Sequence value inside the time unit.
    /// - `machine_id`: Machine identifier.
    ///
    /// # Returns
    /// Encoded ID.
    ///
    /// # Errors
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
    /// # Parameters
    /// - `id`: ID generated by this layout.
    ///
    /// # Returns
    /// Elapsed time units since the start time.
    pub fn extract_elapsed_time(&self, id: u64) -> u64 {
        id >> (self.bits_sequence + self.bits_machine)
    }

    /// Extracts sequence from a Sonyflake-style ID.
    ///
    /// # Parameters
    /// - `id`: ID generated by this layout.
    ///
    /// # Returns
    /// Sequence number.
    pub fn extract_sequence(&self, id: u64) -> u64 {
        let mask = ((1_u64 << self.bits_sequence) - 1) << self.bits_machine;
        (id & mask) >> self.bits_machine
    }

    /// Extracts machine ID from a Sonyflake-style ID.
    ///
    /// # Parameters
    /// - `id`: ID generated by this layout.
    ///
    /// # Returns
    /// Machine identifier.
    pub fn extract_machine_id(&self, id: u64) -> u64 {
        id & ((1_u64 << self.bits_machine) - 1)
    }

    /// Converts a time value into elapsed Sonyflake units.
    ///
    /// # Parameters
    /// - `time`: Time to convert.
    ///
    /// # Returns
    /// Elapsed time units since the start time.
    ///
    /// # Errors
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` is before `start_time`.
    fn elapsed_time_for(&self, time: SystemTime) -> Result<u64, IdError> {
        let elapsed = time
            .duration_since(self.start_time)
            .map_err(|_| IdError::TimeBeforeEpoch)?;
        let elapsed_units = elapsed.as_nanos() / self.time_unit.as_nanos();
        if elapsed_units > u128::from(self.max_elapsed_time()) {
            return Err(IdError::TimestampOverflow {
                timestamp: u64::try_from(elapsed_units).unwrap_or(u64::MAX),
                max: self.max_elapsed_time(),
            });
        }
        Ok(elapsed_units as u64)
    }

    /// Reads the current elapsed time from the configured clock.
    ///
    /// # Returns
    /// Current elapsed time units.
    ///
    /// # Errors
    /// Returns [`IdError::TimeBeforeEpoch`] when the clock is before start
    /// time.
    fn current_elapsed_time(&self) -> Result<u64, IdError> {
        self.elapsed_time_for((self.clock)())
    }

    /// Waits once before retrying allocation through the shared transition.
    ///
    /// The caller reclassifies the clock after this delay so a rollback
    /// observed while waiting is not hidden by an unchecked polling loop.
    fn wait_before_retry(&self) {
        thread::sleep(self.time_unit);
    }
}

impl IdGenerator<u64> for SonyflakeGenerator {
    type Error = IdError;

    /// Generates the next Sonyflake-style ID.
    fn next_id(&self) -> Result<u64, Self::Error> {
        loop {
            let reservation = {
                let mut state = self.state.lock();
                let current = self.current_elapsed_time()?;
                reserve_next(&mut state, current, self.max_sequence())
            };
            match reservation {
                TimeSliceReservation::Allocated(time_slice) => {
                    return self.compose(
                        time_slice.timestamp,
                        time_slice.sequence,
                        self.machine_id,
                    );
                }
                TimeSliceReservation::WaitForNext => {
                    self.wait_before_retry();
                }
                TimeSliceReservation::ClockMovedBackwards {
                    last_timestamp,
                    current_timestamp,
                } => {
                    let skew_units = last_timestamp - current_timestamp;
                    let skew_nanos = u128::from(skew_units)
                        .saturating_mul(self.time_unit.as_nanos());
                    let skew_millis = u64::try_from(skew_nanos / 1_000_000)
                        .unwrap_or(u64::MAX);
                    return Err(IdError::ClockMovedBackwards {
                        last_timestamp,
                        current_timestamp,
                        skew_millis,
                        max_skew_millis: 0,
                    });
                }
            }
        }
    }
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Qubit snowflake generator.

use std::sync::Arc;
use std::thread;
use std::time::{
    Duration,
    SystemTime,
    UNIX_EPOCH,
};

use parking_lot::Mutex;

use super::constants::{
    DEFAULT_MAX_SKEW_MILLIS,
    DEFAULT_QUBIT_EPOCH_MILLIS,
};
use super::time_slice::TimeSlice;
use super::{
    IdMode,
    QubitSnowflakeBuilder,
    TimestampPrecision,
};
use crate::{
    IdError,
    IdGenerator,
};

/// Qubit Snowflake generator.
///
/// This generator uses the Qubit fixed-header layout, including mode and
/// precision bits. The default constructor uses sequential mode, second
/// precision, host `0`, and epoch `2018-12-02T00:00:00Z`.
///
/// The generator is thread-safe and guarantees that successful calls on one
/// instance return distinct IDs. Share one instance inside a process, and use
/// an exclusive host identifier for every concurrently running instance in the
/// same ID namespace.
///
/// The first generation call skips the time slice observed at startup. This
/// can block for at most approximately one configured time slice while the
/// clock advances normally. It prevents reuse after a previous instance with
/// the same host and epoch has stopped, provided the machine clock has not
/// moved backwards between the two instance lifetimes.
pub struct QubitSnowflakeGenerator {
    builder: QubitSnowflakeBuilder,
    epoch: SystemTime,
    max_skew_millis: u64,
    clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    state: Mutex<Option<TimeSlice>>,
}

impl QubitSnowflakeGenerator {
    /// Creates a generator with Qubit defaults.
    ///
    /// # Parameters
    /// - `host`: Host identifier in `0..=511`.
    ///
    /// # Returns
    /// A configured generator.
    ///
    /// # Errors
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit in the host
    /// field.
    pub fn new(host: u64) -> Result<Self, IdError> {
        Self::with_options(
            IdMode::Sequential,
            TimestampPrecision::Second,
            host,
            UNIX_EPOCH + Duration::from_millis(DEFAULT_QUBIT_EPOCH_MILLIS),
        )
    }

    /// Creates a generator with an explicit layout and epoch.
    ///
    /// # Parameters
    /// - `mode`: ID ordering mode.
    /// - `precision`: Timestamp precision.
    /// - `host`: Host identifier in `0..=511`.
    /// - `epoch`: Timestamp origin.
    ///
    /// # Returns
    /// A configured generator using the system clock.
    ///
    /// # Errors
    /// Returns [`IdError::HostOutOfRange`] when `host` is invalid.
    pub fn with_options(
        mode: IdMode,
        precision: TimestampPrecision,
        host: u64,
        epoch: SystemTime,
    ) -> Result<Self, IdError> {
        Self::with_clock(
            mode,
            precision,
            host,
            epoch,
            DEFAULT_MAX_SKEW_MILLIS,
            SystemTime::now,
        )
    }

    /// Creates a generator with an explicit clock.
    ///
    /// This constructor is useful for deterministic tests and for embedding the
    /// generator in systems that already provide a clock abstraction.
    ///
    /// # Parameters
    /// - `mode`: ID ordering mode.
    /// - `precision`: Timestamp precision.
    /// - `host`: Host identifier in `0..=511`.
    /// - `epoch`: Timestamp origin.
    /// - `max_skew_millis`: Maximum tolerated backwards clock movement in
    ///   milliseconds.
    /// - `clock`: Function returning the current time.
    ///
    /// # Returns
    /// A configured generator.
    ///
    /// # Errors
    /// Returns [`IdError::HostOutOfRange`] when `host` is invalid.
    pub fn with_clock<F>(
        mode: IdMode,
        precision: TimestampPrecision,
        host: u64,
        epoch: SystemTime,
        max_skew_millis: u64,
        clock: F,
    ) -> Result<Self, IdError>
    where
        F: Fn() -> SystemTime + Send + Sync + 'static,
    {
        Ok(Self {
            builder: QubitSnowflakeBuilder::new(mode, precision, host)?,
            epoch,
            max_skew_millis,
            clock: Arc::new(clock),
            state: Mutex::new(None),
        })
    }

    /// Returns the Qubit bit builder.
    ///
    /// # Returns
    /// Builder used to compose and inspect generated IDs.
    pub const fn builder(&self) -> &QubitSnowflakeBuilder {
        &self.builder
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    /// Timestamp origin.
    pub const fn epoch(&self) -> SystemTime {
        self.epoch
    }

    /// Generates an ID for an explicit time and sequence.
    ///
    /// # Parameters
    /// - `time`: Time to encode.
    /// - `sequence`: Sequence value inside the encoded time slice.
    ///
    /// # Returns
    /// Encoded ID.
    ///
    /// # Errors
    /// Returns [`IdError::TimeBeforeEpoch`] if `time` is before the configured
    /// epoch. Returns builder validation errors if the computed timestamp or
    /// provided sequence does not fit.
    pub fn generate_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<u64, IdError> {
        let timestamp = self.timestamp_for(time)?;
        self.builder.build(timestamp, sequence)
    }

    /// Converts a time value into a precision-aware timestamp.
    ///
    /// # Parameters
    /// - `time`: Time to convert.
    ///
    /// # Returns
    /// Elapsed timestamp in the configured precision.
    ///
    /// # Errors
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` is before the epoch.
    fn timestamp_for(&self, time: SystemTime) -> Result<u64, IdError> {
        let elapsed = time
            .duration_since(self.epoch)
            .map_err(|_| IdError::TimeBeforeEpoch)?;
        let timestamp = elapsed.as_millis()
            / u128::from(self.builder.precision().divisor_millis());
        if timestamp > u128::from(self.builder.max_timestamp()) {
            return Err(IdError::TimestampOverflow {
                timestamp: u64::try_from(timestamp).unwrap_or(u64::MAX),
                max: self.builder.max_timestamp(),
            });
        }
        Ok(timestamp as u64)
    }

    /// Reads the current timestamp from the configured clock.
    ///
    /// # Returns
    /// Current timestamp in the configured precision.
    ///
    /// # Errors
    /// Returns [`IdError::TimeBeforeEpoch`] when the clock is before the epoch.
    fn current_timestamp(&self) -> Result<u64, IdError> {
        self.timestamp_for((self.clock)())
    }

    /// Waits until the clock reaches a later timestamp.
    ///
    /// # Parameters
    /// - `last_timestamp`: Timestamp that has exhausted its sequence range.
    ///
    /// # Returns
    /// First observed timestamp greater than `last_timestamp`.
    ///
    /// # Errors
    /// Returns [`IdError::TimeBeforeEpoch`] when the clock is before the epoch.
    fn wait_for_next_timestamp(
        &self,
        last_timestamp: u64,
    ) -> Result<u64, IdError> {
        let mut timestamp = self.current_timestamp()?;
        while timestamp <= last_timestamp {
            thread::sleep(Duration::from_millis(
                self.builder.precision().wait_duration_millis(),
            ));
            timestamp = self.current_timestamp()?;
        }
        Ok(timestamp)
    }
}

impl IdGenerator<u64> for QubitSnowflakeGenerator {
    type Error = IdError;

    /// Generates the next Qubit snowflake ID.
    ///
    /// Timestamp and sequence pairs are reserved while holding the generator
    /// mutex. When the current sequence range is exhausted, this method
    /// releases the mutex, waits for a later time slice, and then competes
    /// for a new reservation. The method can therefore block for
    /// approximately one time slice while the clock advances normally, or
    /// longer while tolerating a configured backwards clock skew.
    fn next_id(&self) -> Result<u64, Self::Error> {
        loop {
            let mut state = self.state.lock();
            let timestamp = self.current_timestamp()?;

            let Some(time_slice) = state.as_mut() else {
                *state = Some(TimeSlice::with_sequence(
                    timestamp,
                    self.builder.max_sequence(),
                ));
                drop(state);
                self.wait_for_next_timestamp(timestamp)?;
                continue;
            };

            if time_slice.timestamp > timestamp {
                let skew = time_slice.timestamp - timestamp;
                let skew_millis =
                    skew * self.builder.precision().divisor_millis();
                if skew_millis > self.max_skew_millis {
                    return Err(IdError::ClockMovedBackwards {
                        last_timestamp: time_slice.timestamp,
                        current_timestamp: timestamp,
                        skew_millis,
                        max_skew_millis: self.max_skew_millis,
                    });
                }
                drop(state);
                thread::sleep(Duration::from_millis(skew_millis));
                continue;
            }

            if timestamp > time_slice.timestamp {
                *time_slice = TimeSlice::new(timestamp);
                drop(state);
                return self.builder.build(timestamp, 0);
            }

            if time_slice.sequence == self.builder.max_sequence() {
                let exhausted_timestamp = time_slice.timestamp;
                drop(state);
                self.wait_for_next_timestamp(exhausted_timestamp)?;
                continue;
            }

            time_slice.sequence += 1;
            let sequence = time_slice.sequence;
            drop(state);
            return self.builder.build(timestamp, sequence);
        }
    }
}

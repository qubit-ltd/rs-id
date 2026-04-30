/*******************************************************************************
 *
 *    Copyright (c) 2026.
 *    Haixing Hu, Qubit Co. Ltd.
 *
 *    All rights reserved.
 *
 ******************************************************************************/
//! Java-compatible Qubit snowflake generator.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::constants::{DEFAULT_MAX_SKEW_MILLIS, DEFAULT_QUBIT_EPOCH_MILLIS};
use crate::time_slice::TimeSlice;
use crate::{IdError, IdGenerator, IdMode, QubitSnowflakeBuilder, TimestampPrecision};

/// Java-compatible Snowflake generator used by `java-common/common-id`.
///
/// This generator preserves the Java bit layout, including mode and precision
/// bits. The default constructor uses sequential mode, second precision, host
/// `0`, and epoch `2018-12-02T00:00:00Z`.
pub struct QubitSnowflakeGenerator {
    builder: QubitSnowflakeBuilder,
    epoch: SystemTime,
    max_skew_millis: u64,
    clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    state: Mutex<TimeSlice>,
}

impl QubitSnowflakeGenerator {
    /// Creates a generator with Java-compatible defaults.
    ///
    /// # Parameters
    /// - `host`: Host identifier in `0..=511`.
    ///
    /// # Returns
    /// A configured generator.
    ///
    /// # Errors
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit in the
    /// Java-compatible host field.
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
            state: Mutex::new(TimeSlice::new(0)),
        })
    }

    /// Returns the Java-compatible bit builder.
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
    pub fn generate_at(&self, time: SystemTime, sequence: u64) -> Result<u64, IdError> {
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
        let timestamp = elapsed.as_millis() / u128::from(self.builder.precision().divisor_millis());
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
    fn wait_for_next_timestamp(&self, last_timestamp: u64) -> Result<u64, IdError> {
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

    /// Generates the next Java-compatible Qubit snowflake ID.
    fn next_id(&self) -> Result<u64, Self::Error> {
        loop {
            let mut state = self
                .state
                .lock()
                .expect("generator state mutex should not be poisoned");
            let mut timestamp = self.current_timestamp()?;

            if state.timestamp > timestamp {
                let skew = state.timestamp - timestamp;
                let skew_millis = skew * self.builder.precision().divisor_millis();
                if skew_millis > self.max_skew_millis {
                    return Err(IdError::ClockMovedBackwards {
                        last_timestamp: state.timestamp,
                        current_timestamp: timestamp,
                        skew_millis,
                        max_skew_millis: self.max_skew_millis,
                    });
                }
                drop(state);
                thread::sleep(Duration::from_millis(skew_millis));
                continue;
            }

            let sequence = if timestamp == state.timestamp {
                let next_sequence = (state.sequence + 1) & self.builder.max_sequence();
                if next_sequence == 0 {
                    drop(state);
                    timestamp = self.wait_for_next_timestamp(timestamp)?;
                    let mut state = self
                        .state
                        .lock()
                        .expect("generator state mutex should not be poisoned");
                    state.timestamp = timestamp;
                    state.sequence = 0;
                    return self.builder.build(timestamp, 0);
                }
                next_sequence
            } else {
                0
            };

            state.timestamp = timestamp;
            state.sequence = sequence;
            return self.builder.build(timestamp, sequence);
        }
    }
}

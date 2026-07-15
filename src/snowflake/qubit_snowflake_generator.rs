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
};

use parking_lot::Mutex;

use super::QubitSnowflakeLayout;
use super::qubit_snowflake_generator_builder::QubitSnowflakeGeneratorBuilder;
use super::time_slice::TimeSlice;
use super::time_slice_reservation::{
    TimeSliceReservation,
    reserve_next,
};
use crate::{
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
/// The generator is thread-safe. Successful [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] calls on one shared live instance never return
/// the same ID. A process should share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive host identifier when its layout and epoch can produce IDs in
/// the same namespace.
///
/// The first generation call allocates sequence zero in the currently observed
/// time slice without waiting. Allocation state is not persisted, so replacing
/// an instance with the same host, layout, and epoch inside one time slice can
/// repeat IDs. Applications that reuse hosts across restarts must coordinate
/// host leases, wait outside the generator, or persist allocation state.
///
/// # Blocking and clock behavior
/// A call after sequence exhaustion can block for approximately one configured
/// time slice while the clock advances normally. A stalled clock can block
/// indefinitely. A backwards clock movement within `max_skew_millis` is
/// retried after waiting; a larger movement returns
/// [`IdError::ClockMovedBackwards`].
pub struct QubitSnowflakeGenerator {
    layout: QubitSnowflakeLayout,
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
        Self::builder(host).build()
    }

    /// Creates a configurable generator builder for the specified host.
    ///
    /// Host validation is performed when
    /// [`QubitSnowflakeGeneratorBuilder::build`] is called.
    #[must_use]
    pub fn builder(host: u64) -> QubitSnowflakeGeneratorBuilder {
        QubitSnowflakeGeneratorBuilder::new(host)
    }

    /// Constructs a generator from a validated builder configuration.
    pub(super) fn from_config(
        layout: QubitSnowflakeLayout,
        epoch: SystemTime,
        max_skew_millis: u64,
        clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    ) -> Self {
        Self {
            layout,
            epoch,
            max_skew_millis,
            clock,
            state: Mutex::new(None),
        }
    }

    /// Returns the Qubit bit layout.
    ///
    /// # Returns
    /// Layout used to compose generated IDs.
    pub const fn layout(&self) -> &QubitSnowflakeLayout {
        &self.layout
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    /// Timestamp origin.
    pub const fn epoch(&self) -> SystemTime {
        self.epoch
    }

    /// Returns the maximum tolerated backwards clock movement.
    ///
    /// # Returns
    /// Maximum clock skew in milliseconds.
    pub const fn max_skew_millis(&self) -> u64 {
        self.max_skew_millis
    }

    /// Generates an ID for an explicit time and sequence.
    ///
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
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
    /// epoch. Returns [`IdError::TimestampOverflow`] or
    /// [`IdError::SequenceOverflow`] when the computed timestamp or provided
    /// sequence does not fit the layout.
    pub fn generate_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<u64, IdError> {
        let timestamp = self.timestamp_for(time)?;
        self.layout.compose(timestamp, sequence)
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
            / u128::from(self.layout.precision().divisor_millis());
        if timestamp > u128::from(self.layout.max_timestamp()) {
            return Err(IdError::TimestampOverflow {
                timestamp: u64::try_from(timestamp).unwrap_or(u64::MAX),
                max: self.layout.max_timestamp(),
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

    /// Waits once before retrying allocation through the shared transition.
    ///
    /// The caller must read and classify the clock again after this delay so a
    /// rollback observed while waiting cannot bypass the configured policy.
    fn wait_before_retry(&self) {
        thread::sleep(Duration::from_millis(
            self.layout.precision().wait_duration_millis(),
        ));
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
            let reservation = {
                let mut state = self.state.lock();
                let timestamp = self.current_timestamp()?;
                reserve_next(&mut state, timestamp, self.layout.max_sequence())
            };
            match reservation {
                TimeSliceReservation::Allocated(time_slice) => {
                    return self
                        .layout
                        .compose(time_slice.timestamp, time_slice.sequence);
                }
                TimeSliceReservation::WaitForNext => {
                    self.wait_before_retry();
                }
                TimeSliceReservation::ClockMovedBackwards {
                    last_timestamp,
                    current_timestamp,
                } => {
                    let skew = last_timestamp - current_timestamp;
                    let skew_millis =
                        skew * self.layout.precision().divisor_millis();
                    if skew_millis > self.max_skew_millis {
                        return Err(IdError::ClockMovedBackwards {
                            last_timestamp,
                            current_timestamp,
                            skew_millis,
                            max_skew_millis: self.max_skew_millis,
                        });
                    }
                    thread::sleep(Duration::from_millis(skew_millis));
                }
            }
        }
    }
}

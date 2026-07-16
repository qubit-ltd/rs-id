// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Classic 41/10/12 Snowflake generator.

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

use super::RestartPolicy;
use super::internal::{
    ClockObservation,
    GenerationState,
    block_until_generated,
};
use super::snowflake_generator_builder::SnowflakeGeneratorBuilder;
use crate::{
    GenerationOutcome,
    IdError,
    IdGenerator,
};

const TIMESTAMP_BITS: u8 = 41;
const NODE_BITS: u8 = 10;
const SEQUENCE_BITS: u8 = 12;
const MAX_NODE_ID: u64 = (1_u64 << NODE_BITS) - 1;

/// Classic Snowflake generator using 41 timestamp, 10 node, and 12 sequence
/// bits.
///
/// # Uniqueness
///
/// The generator is thread-safe. Successful [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] calls on one shared live instance never return
/// the same ID. A process should share one instance for each ID namespace.
/// Every concurrently running instance across processes and servers must have
/// an exclusive node identifier when its epoch can produce IDs in the same
/// namespace.
///
/// The default [`RestartPolicy::Immediate`] allocates sequence zero in the
/// currently observed millisecond without waiting. Allocation state is not
/// persisted. State loss or replacement can repeat an ID only when the
/// instances use the same effective identity (`node_id`), layout, and
/// reference time (`epoch`), allocate in the same logical millisecond, and use
/// overlapping sequence ranges.
///
/// [`RestartPolicy::WaitNextSlice`] waits until after the first observed
/// millisecond. It protects sequential replacement only; it does not
/// coordinate concurrent same-identity instances, which can cross the fence
/// together and allocate overlapping sequence ranges. Such deployments
/// require external exclusivity.
///
/// # Blocking and clock behavior
///
/// [`IdGenerator::try_next_id`] performs one attempt and never sleeps for clock
/// progress or invokes the configured sleeper, although it can briefly contend
/// for the internal mutex. [`IdGenerator::next_id`] and
/// [`IdGenerator::next_string`] wait across retry outcomes. They may wait
/// indefinitely when the wall clock stalls or
/// the injected sleeper does not cause wall time to progress. A backwards
/// clock movement returns
/// [`IdError::ClockMovedBackwards`] immediately.
pub struct SnowflakeGenerator {
    /// Node identifier encoded in generated IDs.
    node_id: u64,
    /// Timestamp origin used by encoded timestamps.
    epoch: SystemTime,
    /// Wall clock sampled by allocation attempts.
    wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    blocking_sleeper: Arc<dyn BlockingSleeper>,
    /// Synchronized raw-clock and sequence allocation state.
    state: Mutex<GenerationState>,
}

impl SnowflakeGenerator {
    /// Creates a classic Snowflake generator with the default Qubit epoch.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier in `0..=1023`.
    ///
    /// # Returns
    ///
    /// A configured generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] when `node_id` does not fit in 10
    /// bits.
    #[inline(always)]
    pub fn new(node_id: u64) -> Result<Self, IdError> {
        Self::builder(node_id).build()
    }

    /// Creates a configurable builder for the specified node identifier.
    ///
    /// Node validation is performed when [`SnowflakeGeneratorBuilder::build`]
    /// is called.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable classic Snowflake generator builder.
    #[must_use]
    #[inline(always)]
    pub fn builder(node_id: u64) -> SnowflakeGeneratorBuilder {
        SnowflakeGeneratorBuilder::new(node_id)
    }

    /// Constructs a generator from a complete builder configuration.
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to encode in generated IDs.
    /// * `epoch` - Timestamp origin used by the generator.
    /// * `restart_policy` - Policy controlling the first allocation.
    /// * `wall_clock` - Wall clock sampled during allocation.
    /// * `blocking_sleeper` - Sleeper used by blocking generation.
    ///
    /// # Returns
    ///
    /// A generator containing the complete builder configuration.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] when `node_id` does not fit in 10
    /// bits.
    pub(super) fn from_config(
        node_id: u64,
        epoch: SystemTime,
        restart_policy: RestartPolicy,
        wall_clock: Arc<dyn WallClock>,
        blocking_sleeper: Arc<dyn BlockingSleeper>,
    ) -> Result<Self, IdError> {
        if node_id > MAX_NODE_ID {
            return Err(IdError::NodeOutOfRange {
                node_id,
                max: MAX_NODE_ID,
            });
        }
        Ok(Self {
            node_id,
            epoch,
            wall_clock,
            blocking_sleeper,
            state: Mutex::new(GenerationState::new(restart_policy)),
        })
    }

    /// Returns the configured node identifier.
    ///
    /// # Returns
    ///
    /// Node identifier.
    #[inline(always)]
    pub const fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Returns the configured epoch.
    ///
    /// # Returns
    ///
    /// Timestamp origin.
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
        self.epoch
    }

    /// Returns the maximum representable timestamp.
    ///
    /// # Returns
    ///
    /// Maximum timestamp in milliseconds since the epoch.
    #[inline(always)]
    pub const fn max_timestamp(&self) -> u64 {
        (1_u64 << TIMESTAMP_BITS) - 1
    }

    /// Returns the maximum representable sequence.
    ///
    /// # Returns
    ///
    /// Maximum sequence number.
    #[inline(always)]
    pub const fn max_sequence(&self) -> u64 {
        (1_u64 << SEQUENCE_BITS) - 1
    }

    /// Composes an ID from timestamp and sequence parts.
    ///
    /// This method is stateless. Repeating its inputs repeats the ID, so it
    /// provides no uniqueness guarantee.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - Milliseconds elapsed since the epoch.
    /// * `sequence` - Sequence value inside the timestamp millisecond.
    ///
    /// # Returns
    ///
    /// Encoded ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimestampOverflow`] or [`IdError::SequenceOverflow`]
    /// when a part does not fit the classic Snowflake layout.
    pub fn compose(
        &self,
        timestamp: u64,
        sequence: u64,
    ) -> Result<u64, IdError> {
        if timestamp > self.max_timestamp() {
            return Err(IdError::TimestampOverflow {
                timestamp,
                max: self.max_timestamp(),
            });
        }
        if sequence > self.max_sequence() {
            return Err(IdError::SequenceOverflow {
                sequence,
                max: self.max_sequence(),
            });
        }
        Ok((timestamp << (NODE_BITS + SEQUENCE_BITS))
            | (self.node_id << SEQUENCE_BITS)
            | sequence)
    }

    /// Extracts the timestamp part from an ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID generated by this layout.
    ///
    /// # Returns
    ///
    /// Milliseconds elapsed since the epoch.
    #[inline(always)]
    pub const fn extract_timestamp(&self, id: u64) -> u64 {
        id >> (NODE_BITS + SEQUENCE_BITS)
    }

    /// Extracts the node identifier from an ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID generated by this layout.
    ///
    /// # Returns
    ///
    /// Node identifier.
    #[inline(always)]
    pub const fn extract_node_id(&self, id: u64) -> u64 {
        (id >> SEQUENCE_BITS) & MAX_NODE_ID
    }

    /// Extracts the sequence number from an ID.
    ///
    /// # Arguments
    ///
    /// * `id` - ID generated by this layout.
    ///
    /// # Returns
    ///
    /// Sequence number.
    #[inline(always)]
    pub const fn extract_sequence(&self, id: u64) -> u64 {
        id & ((1_u64 << SEQUENCE_BITS) - 1)
    }

    /// Converts a clock time into a raw millisecond observation.
    ///
    /// # Arguments
    ///
    /// * `time` - Time to convert.
    ///
    /// # Returns
    ///
    /// Raw elapsed time and encoded milliseconds since the epoch.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when `time` is before the epoch.
    fn observation_for(
        &self,
        time: SystemTime,
    ) -> Result<ClockObservation, IdError> {
        ClockObservation::from_time(
            time,
            self.epoch,
            Duration::from_millis(1),
            self.max_timestamp(),
        )
    }
}

impl IdGenerator for SnowflakeGenerator {
    type Id = u64;
    type Error = IdError;

    /// Performs one classic Snowflake allocation attempt without sleeping.
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
                .compose(time_slice.timestamp, time_slice.sequence)
                .map(GenerationOutcome::Generated),
            GenerationOutcome::RetryAfter(duration) => {
                Ok(GenerationOutcome::RetryAfter(duration))
            }
        }
    }

    /// Generates the next classic Snowflake ID.
    ///
    /// This method blocks across retryable sequence exhaustion.
    ///
    /// # Returns
    ///
    /// The next generated classic Snowflake ID.
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

    /// Formats a classic Snowflake ID as unsigned decimal text.
    ///
    /// # Arguments
    ///
    /// * `id` - Classic Snowflake ID to format.
    ///
    /// # Returns
    ///
    /// Unsigned decimal text for `id`.
    #[inline(always)]
    fn format_id(&self, id: &Self::Id) -> String {
        id.to_string()
    }
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for the classic Snowflake generator.

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

use super::constants::DEFAULT_QUBIT_EPOCH_MILLIS;
use super::internal::{
    default_blocking_sleeper,
    default_wall_clock,
};
use super::{
    RestartPolicy,
    SnowflakeGenerator,
};
use crate::IdError;

/// Configures and constructs a [`SnowflakeGenerator`].
///
/// Unspecified options use the default Qubit epoch,
/// [`RestartPolicy::Immediate`], and standard clock and sleeper capabilities.
pub struct SnowflakeGeneratorBuilder {
    /// Node identifier encoded in generated IDs.
    node_id: u64,
    /// Timestamp origin.
    epoch: SystemTime,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled by the generator.
    wall_clock: Arc<dyn WallClock>,
    /// Sleeper used only by blocking generation.
    blocking_sleeper: Arc<dyn BlockingSleeper>,
}

impl SnowflakeGeneratorBuilder {
    /// Creates a builder for the specified node identifier.
    ///
    /// Node validation is deferred until [`Self::build`].
    ///
    /// # Arguments
    ///
    /// * `node_id` - Node identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A builder using the default Qubit epoch, immediate restart policy, and
    /// standard clocks.
    #[inline]
    pub(crate) fn new(node_id: u64) -> Self {
        Self {
            node_id,
            epoch: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_QUBIT_EPOCH_MILLIS),
            restart_policy: RestartPolicy::Immediate,
            wall_clock: default_wall_clock(),
            blocking_sleeper: default_blocking_sleeper(),
        }
    }

    /// Sets the timestamp origin used by generated IDs.
    ///
    /// # Arguments
    ///
    /// * `epoch` - Timestamp origin to configure.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn epoch(mut self, epoch: SystemTime) -> Self {
        self.epoch = epoch;
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

    /// Sets the wall clock sampled by allocation attempts.
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

    /// Validates the configuration and constructs a generator.
    ///
    /// # Returns
    ///
    /// A configured classic Snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] when the node identifier does not
    /// fit the classic 10-bit node field.
    #[inline(always)]
    pub fn build(self) -> Result<SnowflakeGenerator, IdError> {
        SnowflakeGenerator::from_config(
            self.node_id,
            self.epoch,
            self.restart_policy,
            self.wall_clock,
            self.blocking_sleeper,
        )
    }
}

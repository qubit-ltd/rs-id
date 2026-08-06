// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for classic Snowflake generators.

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
    DEFAULT_SNOWFLAKE_EPOCH_MILLIS,
    SnowflakeCore,
    default_timer,
    default_wall_clock,
};
use super::{
    RestartPolicy,
    SnowflakeGenerator,
    SnowflakeLayout,
};
use crate::IdError;

/// Configures synchronous or asynchronous classic Snowflake generators.
#[must_use = "builders do nothing unless built"]
pub struct SnowflakeGeneratorBuilder {
    /// Node identifier encoded in generated IDs.
    node_id: u64,
    /// Timestamp origin.
    epoch: SystemTime,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled by the generator.
    wall_clock: Arc<dyn WallClock>,
    /// Timer used by blocking or asynchronous waits.
    timer: Arc<dyn Timer>,
}

impl SnowflakeGeneratorBuilder {
    /// Creates a builder for a node identifier.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A builder initialized with the default epoch, clocks, and restart
    /// policy.
    #[inline]
    pub(crate) fn new(node_id: u64) -> Self {
        Self {
            node_id,
            epoch: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_SNOWFLAKE_EPOCH_MILLIS),
            restart_policy: RestartPolicy::WaitNextSlice,
            wall_clock: default_wall_clock(),
            timer: default_timer(),
        }
    }

    /// Sets the timestamp origin used by generated IDs.
    ///
    /// # Parameters
    ///
    /// * `epoch` - Timestamp origin represented by timestamp zero.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn epoch(mut self, epoch: SystemTime) -> Self {
        self.epoch = epoch;
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

    /// Sets the wall clock sampled by allocation attempts.
    ///
    /// # Parameters
    ///
    /// * `wall_clock` - Wall clock sampled inside the allocation lock.
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

    /// Validates the configuration and constructs a synchronous generator.
    ///
    /// # Returns
    ///
    /// A synchronous classic Snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] or
    /// [`IdError::ExpirationTimeOverflow`] for an invalid configuration, or
    /// [`IdError::GeneratorExpired`] when the configured wall clock has
    /// reached the expiration boundary.
    #[inline]
    pub fn build(self) -> Result<SnowflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(SnowflakeGenerator::from_core(core, timer))
    }

    /// Converts the builder into a validated shared core and timer.
    ///
    /// # Returns
    ///
    /// The validated allocation core and configured timer.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] or
    /// [`IdError::ExpirationTimeOverflow`] for an invalid configuration, or
    /// [`IdError::GeneratorExpired`] when the configured wall clock has
    /// reached the expiration boundary.
    fn into_core(
        self,
    ) -> Result<(SnowflakeCore<SnowflakeLayout>, Arc<dyn Timer>), IdError> {
        let layout = SnowflakeLayout::new(self.node_id)?;
        let expires_at = layout.expires_at(self.epoch)?;
        let current_time = self.wall_clock.now();
        if current_time >= expires_at {
            return Err(IdError::GeneratorExpired {
                observed_at: current_time,
                expires_at,
            });
        }
        let core = SnowflakeCore::new(
            layout,
            self.epoch,
            expires_at,
            Duration::ZERO,
            self.restart_policy,
            self.wall_clock,
        );
        Ok((core, self.timer))
    }
}

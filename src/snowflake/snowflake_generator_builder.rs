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
    panic_if_expired,
};
use super::{
    AsyncSnowflakeGenerator,
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
    #[inline]
    pub(crate) fn new(node_id: u64) -> Self {
        Self {
            node_id,
            epoch: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_SNOWFLAKE_EPOCH_MILLIS),
            restart_policy: RestartPolicy::Immediate,
            wall_clock: default_wall_clock(),
            timer: default_timer(),
        }
    }

    /// Sets the timestamp origin used by generated IDs.
    #[inline(always)]
    pub fn epoch(mut self, epoch: SystemTime) -> Self {
        self.epoch = epoch;
        self
    }

    /// Sets the first-allocation behavior used after construction.
    #[inline(always)]
    pub fn restart_policy(mut self, restart_policy: RestartPolicy) -> Self {
        self.restart_policy = restart_policy;
        self
    }

    /// Sets the wall clock sampled by allocation attempts.
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
    /// Returns [`IdError::NodeOutOfRange`] or
    /// [`IdError::ExpirationTimeOverflow`] for an invalid configuration.
    ///
    /// # Panics
    ///
    /// Panics when the configured wall clock has reached the expiration
    /// boundary.
    #[inline]
    pub fn build(self) -> Result<SnowflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(SnowflakeGenerator::from_core(core, timer))
    }

    /// Validates the configuration and constructs an asynchronous generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] or
    /// [`IdError::ExpirationTimeOverflow`] for an invalid configuration.
    ///
    /// # Panics
    ///
    /// Panics when the configured wall clock has reached the expiration
    /// boundary.
    #[inline]
    pub fn build_async(self) -> Result<AsyncSnowflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(AsyncSnowflakeGenerator::from_core(core, timer))
    }

    /// Converts the builder into a validated shared core and timer.
    fn into_core(
        self,
    ) -> Result<(SnowflakeCore<SnowflakeLayout>, Arc<dyn Timer>), IdError> {
        let layout = SnowflakeLayout::new(self.node_id)?;
        let expires_at = layout.expires_at(self.epoch)?;
        let current_time = self.wall_clock.now();
        panic_if_expired("classic Snowflake", current_time, expires_at);
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

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

use super::super::RestartPolicy;
use super::super::internal::{
    DEFAULT_SNOWFLAKE_EPOCH_MILLIS,
    SnowflakeCore,
    default_timer,
    default_wall_clock,
    validate_generator_epoch,
    validate_generator_lifetime,
};
use super::{
    ClassicalSnowflakeGenerator,
    ClassicalSnowflakeLayout,
};
use crate::IdGenerationError;

/// Configures synchronous or asynchronous classic Snowflake generators.
#[must_use = "builders do nothing unless built"]
pub struct ClassicalSnowflakeGeneratorBuilder {
    /// Node identifier encoded in generated IDs.
    node_id: u64,
    /// Timestamp origin.
    epoch: SystemTime,
    /// Maximum tolerated raw wall-clock rollback.
    max_clock_skew: Duration,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled by the generator.
    wall_clock: Arc<dyn WallClock>,
    /// Timer used by blocking or asynchronous waits.
    timer: Arc<dyn Timer>,
}

impl ClassicalSnowflakeGeneratorBuilder {
    /// Creates a builder for a node identifier.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A builder initialized with the default epoch, zero clock-skew
    /// tolerance, clocks, and immediate restart policy.
    #[inline]
    pub(crate) fn new(node_id: u64) -> Self {
        Self {
            node_id,
            epoch: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_SNOWFLAKE_EPOCH_MILLIS),
            max_clock_skew: Duration::ZERO,
            restart_policy: RestartPolicy::Immediate,
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

    /// Sets the maximum tolerated raw wall-clock rollback.
    ///
    /// # Parameters
    ///
    /// * `max_clock_skew` - Largest raw rollback that may be retried.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn max_clock_skew(mut self, max_clock_skew: Duration) -> Self {
        self.max_clock_skew = max_clock_skew;
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

    /// Validates the configuration and constructs a generator.
    ///
    /// # Returns
    ///
    /// A synchronous classic Snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::NodeOutOfRange`] or
    /// [`IdGenerationError::ExpirationTimeOverflow`] for an invalid
    /// configuration, [`IdGenerationError::EpochAhead`] when the configured
    /// epoch is later than the wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when the configured wall clock
    /// has reached the expiration boundary.
    #[inline]
    pub fn build(
        self,
    ) -> Result<ClassicalSnowflakeGenerator, IdGenerationError> {
        let (core, timer) = self.into_core()?;
        Ok(ClassicalSnowflakeGenerator::from_core(core, timer))
    }

    /// Converts the builder into a validated shared core and timer.
    ///
    /// # Returns
    ///
    /// The validated allocation core and configured timer.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::NodeOutOfRange`] or
    /// [`IdGenerationError::ExpirationTimeOverflow`] for an invalid
    /// configuration, [`IdGenerationError::EpochAhead`] when the configured
    /// epoch is later than the wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when the configured wall clock
    /// has reached the expiration boundary.
    fn into_core(
        self,
    ) -> Result<
        (SnowflakeCore<ClassicalSnowflakeLayout>, Arc<dyn Timer>),
        IdGenerationError,
    > {
        let layout = ClassicalSnowflakeLayout::new(self.node_id)?;
        let current_time = self.wall_clock.now();
        validate_generator_epoch(self.epoch, current_time)?;
        let expires_at = layout.expires_at(self.epoch)?;
        validate_generator_lifetime(self.epoch, expires_at, current_time)?;
        let core = SnowflakeCore::new(
            layout,
            self.epoch,
            expires_at,
            self.max_clock_skew,
            self.restart_policy,
            self.wall_clock,
        );
        Ok((core, self.timer))
    }
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Builder for the Qubit snowflake generator.

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
use super::constants::DEFAULT_MAX_CLOCK_SKEW;
use super::snowflake_generator::SnowflakeGenerator;
use super::{
    IdMode,
    SnowflakeLayout,
    TimestampPrecision,
};
use crate::IdGenerationError;

/// Configures and constructs a [`SnowflakeGenerator`].
///
/// The required host is supplied when the builder is created. Unspecified
/// options use Qubit defaults: sequential mode, second precision, epoch
/// `2018-12-02T00:00:00Z`, the default clock-skew tolerance,
/// [`RestartPolicy::Immediate`], and standard clock and timer capabilities.
#[must_use = "builders do nothing unless built"]
pub struct SnowflakeGeneratorBuilder {
    /// ID ordering mode encoded in generated IDs.
    mode: IdMode,
    /// Timestamp precision and corresponding field allocation.
    precision: TimestampPrecision,
    /// Host identifier encoded in generated IDs.
    host: u64,
    /// Timestamp origin used by encoded timestamps.
    epoch: SystemTime,
    /// Maximum tolerated raw wall-clock rollback.
    max_clock_skew: Duration,
    /// First-allocation policy.
    restart_policy: RestartPolicy,
    /// Wall clock sampled by allocation attempts.
    wall_clock: Arc<dyn WallClock>,
    /// Timer used by blocking or asynchronous retry waits.
    timer: Arc<dyn Timer>,
}

impl SnowflakeGeneratorBuilder {
    /// Creates a builder for the specified host.
    ///
    /// Host validation is deferred until [`Self::build`].
    ///
    /// # Parameters
    ///
    /// * `host` - Host identifier to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// A builder using the Qubit defaults and standard clocks.
    #[inline]
    pub(crate) fn new(host: u64) -> Self {
        Self {
            mode: IdMode::Sequential,
            precision: TimestampPrecision::Second,
            host,
            epoch: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_SNOWFLAKE_EPOCH_MILLIS),
            max_clock_skew: DEFAULT_MAX_CLOCK_SKEW,
            restart_policy: RestartPolicy::Immediate,
            wall_clock: default_wall_clock(),
            timer: default_timer(),
        }
    }

    /// Sets the encoded ID ordering mode.
    ///
    /// # Parameters
    ///
    /// * `mode` - Ordering mode to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn mode(mut self, mode: IdMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the timestamp precision and corresponding field allocation.
    ///
    /// # Parameters
    ///
    /// * `precision` - Precision and field allocation to use.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[inline(always)]
    pub fn precision(mut self, precision: TimestampPrecision) -> Self {
        self.precision = precision;
        self
    }

    /// Sets the timestamp origin used by generated IDs.
    ///
    /// # Parameters
    ///
    /// * `epoch` - Timestamp origin to configure.
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
    /// * `restart_policy` - Policy controlling the first allocation.
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
    /// * `wall_clock` - Shared wall clock to sample.
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
    /// * `timer` - Shared timer used for retry delays.
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
    /// A configured Qubit snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::HostOutOfRange`] when the configured host
    /// does not fit the Qubit host field, or
    /// [`IdGenerationError::ExpirationTimeOverflow`] when the exclusive
    /// expiration cannot be represented, or
    /// [`IdGenerationError::EpochAhead`] when the configured epoch is later
    /// than the wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when the configured wall clock
    /// is equal to or later than that boundary.
    #[inline]
    pub fn build(self) -> Result<SnowflakeGenerator, IdGenerationError> {
        let (core, timer) = self.into_core()?;
        Ok(SnowflakeGenerator::from_core(core, timer))
    }

    /// Converts this builder into a validated shared core and timer.
    ///
    /// # Returns
    ///
    /// The validated allocation core and configured timer.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::HostOutOfRange`] when the configured host
    /// does not fit the Qubit host field,
    /// [`IdGenerationError::ExpirationTimeOverflow`] when the
    /// exclusive expiration cannot be represented, or
    /// [`IdGenerationError::EpochAhead`] when the configured epoch is later
    /// than the wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when the configured wall clock
    /// is equal to or later than that boundary.
    fn into_core(
        self,
    ) -> Result<
        (SnowflakeCore<SnowflakeLayout>, Arc<dyn Timer>),
        IdGenerationError,
    > {
        let layout =
            SnowflakeLayout::new(self.mode, self.precision, self.host)?;
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

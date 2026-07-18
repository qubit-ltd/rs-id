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

use super::AsyncQubitSnowflakeGenerator;
use super::constants::DEFAULT_MAX_CLOCK_SKEW;
use super::internal::{
    DEFAULT_SNOWFLAKE_EPOCH_MILLIS,
    SnowflakeCore,
    default_timer,
    default_wall_clock,
};
use super::qubit_snowflake_generator::QubitSnowflakeGenerator;
use super::{
    IdMode,
    QubitSnowflakeLayout,
    RestartPolicy,
    TimestampPrecision,
};
use crate::IdError;

/// Configures and constructs a [`QubitSnowflakeGenerator`].
///
/// The required host is supplied when the builder is created. Unspecified
/// options use Qubit defaults: sequential mode, second precision, epoch
/// `2018-12-02T00:00:00Z`, the default clock-skew tolerance,
/// [`RestartPolicy::Immediate`], and standard clock and timer capabilities.
#[must_use = "builders do nothing unless built"]
pub struct QubitSnowflakeGeneratorBuilder {
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

impl QubitSnowflakeGeneratorBuilder {
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
    /// Returns [`IdError::HostOutOfRange`] when the configured host does not
    /// fit the Qubit host field, or [`IdError::ExpirationTimeOverflow`] when
    /// the exclusive expiration cannot be represented, or
    /// [`IdError::GeneratorExpired`] when the configured wall clock is equal
    /// to or later than that boundary.
    #[inline]
    pub fn build(self) -> Result<QubitSnowflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(QubitSnowflakeGenerator::from_core(core, timer))
    }

    /// Validates the configuration and constructs an asynchronous generator.
    ///
    /// # Returns
    ///
    /// A configured asynchronous Qubit Snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::HostOutOfRange`] when the configured host does not
    /// fit the Qubit host field, or [`IdError::ExpirationTimeOverflow`] when
    /// the exclusive expiration cannot be represented, or
    /// [`IdError::GeneratorExpired`] when the configured wall clock is equal
    /// to or later than that boundary.
    #[inline]
    pub fn build_async(self) -> Result<AsyncQubitSnowflakeGenerator, IdError> {
        let (core, timer) = self.into_core()?;
        Ok(AsyncQubitSnowflakeGenerator::from_core(core, timer))
    }

    /// Converts this builder into a validated shared core and timer.
    fn into_core(
        self,
    ) -> Result<(SnowflakeCore<QubitSnowflakeLayout>, Arc<dyn Timer>), IdError>
    {
        let layout =
            QubitSnowflakeLayout::new(self.mode, self.precision, self.host)?;
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
            self.max_clock_skew,
            self.restart_policy,
            self.wall_clock,
        );
        Ok((core, self.timer))
    }
}

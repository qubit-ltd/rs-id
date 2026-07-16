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
    BlockingSleeper,
    WallClock,
};

use super::constants::{
    DEFAULT_MAX_CLOCK_SKEW,
    DEFAULT_QUBIT_EPOCH_MILLIS,
};
use super::internal::{
    default_blocking_sleeper,
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
/// [`RestartPolicy::Immediate`], and standard clock and sleeper capabilities.
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
    /// Sleeper used only by blocking generation.
    blocking_sleeper: Arc<dyn BlockingSleeper>,
}

impl QubitSnowflakeGeneratorBuilder {
    /// Creates a builder for the specified host.
    ///
    /// Host validation is deferred until [`Self::build`].
    ///
    /// # Arguments
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
                + Duration::from_millis(DEFAULT_QUBIT_EPOCH_MILLIS),
            max_clock_skew: DEFAULT_MAX_CLOCK_SKEW,
            restart_policy: RestartPolicy::Immediate,
            wall_clock: default_wall_clock(),
            blocking_sleeper: default_blocking_sleeper(),
        }
    }

    /// Sets the encoded ID ordering mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - Ordering mode to encode in generated IDs.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn mode(mut self, mode: IdMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the timestamp precision and corresponding field allocation.
    ///
    /// # Arguments
    ///
    /// * `precision` - Precision and field allocation to use.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn precision(mut self, precision: TimestampPrecision) -> Self {
        self.precision = precision;
        self
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

    /// Sets the maximum tolerated raw wall-clock rollback.
    ///
    /// # Arguments
    ///
    /// * `max_clock_skew` - Largest raw rollback that may be retried.
    ///
    /// # Returns
    ///
    /// The updated builder.
    #[must_use]
    #[inline(always)]
    pub fn max_clock_skew(mut self, max_clock_skew: Duration) -> Self {
        self.max_clock_skew = max_clock_skew;
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
    /// A configured Qubit snowflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::HostOutOfRange`] when the configured host does not
    /// fit the Qubit host field.
    #[inline(always)]
    pub fn build(self) -> Result<QubitSnowflakeGenerator, IdError> {
        let layout =
            QubitSnowflakeLayout::new(self.mode, self.precision, self.host)?;
        Ok(QubitSnowflakeGenerator::from_config(
            layout,
            self.epoch,
            self.max_clock_skew,
            self.restart_policy,
            self.wall_clock,
            self.blocking_sleeper,
        ))
    }
}

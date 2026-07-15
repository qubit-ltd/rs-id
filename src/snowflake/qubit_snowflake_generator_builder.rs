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

use super::constants::{
    DEFAULT_MAX_SKEW_MILLIS,
    DEFAULT_QUBIT_EPOCH_MILLIS,
};
use super::qubit_snowflake_generator::QubitSnowflakeGenerator;
use super::{
    IdMode,
    QubitSnowflakeLayout,
    TimestampPrecision,
};
use crate::IdError;

/// Configures and constructs a [`QubitSnowflakeGenerator`].
///
/// The required host is supplied when the builder is created. Unspecified
/// options use Qubit defaults: sequential mode, second precision, epoch
/// `2018-12-02T00:00:00Z`, the default clock-skew tolerance, and the system
/// wall clock.
pub struct QubitSnowflakeGeneratorBuilder {
    mode: IdMode,
    precision: TimestampPrecision,
    host: u64,
    epoch: SystemTime,
    max_skew_millis: u64,
    clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
}

impl QubitSnowflakeGeneratorBuilder {
    /// Creates a builder for the specified host.
    ///
    /// Host validation is deferred until [`Self::build`].
    pub(crate) fn new(host: u64) -> Self {
        Self {
            mode: IdMode::Sequential,
            precision: TimestampPrecision::Second,
            host,
            epoch: UNIX_EPOCH
                + Duration::from_millis(DEFAULT_QUBIT_EPOCH_MILLIS),
            max_skew_millis: DEFAULT_MAX_SKEW_MILLIS,
            clock: Arc::new(SystemTime::now),
        }
    }

    /// Sets the encoded ID ordering mode.
    #[must_use]
    pub fn mode(mut self, mode: IdMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the timestamp precision and corresponding field allocation.
    #[must_use]
    pub fn precision(mut self, precision: TimestampPrecision) -> Self {
        self.precision = precision;
        self
    }

    /// Sets the timestamp origin used by generated IDs.
    #[must_use]
    pub fn epoch(mut self, epoch: SystemTime) -> Self {
        self.epoch = epoch;
        self
    }

    /// Sets the maximum tolerated backwards clock skew in milliseconds.
    #[must_use]
    pub fn max_skew_millis(mut self, max_skew_millis: u64) -> Self {
        self.max_skew_millis = max_skew_millis;
        self
    }

    /// Sets the wall-clock function used by the generator.
    ///
    /// This option supports deterministic tests and applications with an
    /// existing wall-clock abstraction. The function may be called
    /// concurrently by multiple generator clients.
    #[must_use]
    pub fn clock<F>(mut self, clock: F) -> Self
    where
        F: Fn() -> SystemTime + Send + Sync + 'static,
    {
        self.clock = Arc::new(clock);
        self
    }

    /// Validates the configuration and constructs a generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::HostOutOfRange`] when the configured host does not
    /// fit the Qubit host field.
    pub fn build(self) -> Result<QubitSnowflakeGenerator, IdError> {
        let layout =
            QubitSnowflakeLayout::new(self.mode, self.precision, self.host)?;
        Ok(QubitSnowflakeGenerator::from_config(
            layout,
            self.epoch,
            self.max_skew_millis,
            self.clock,
        ))
    }
}

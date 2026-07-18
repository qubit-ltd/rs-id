// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Asynchronous Qubit Snowflake generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use qubit_clock::Timer;

use super::internal::{
    AsyncSnowflake,
    SnowflakeCore,
};
use super::{
    QubitSnowflakeGeneratorBuilder,
    QubitSnowflakeLayout,
};
use crate::{
    AsyncIdGenerator,
    IdError,
    IdGenerationFuture,
};

/// Generates Qubit Snowflake IDs without blocking an asynchronous executor.
///
/// The generator is thread-safe and uses internal synchronization, so one
/// shared instance can serve concurrent tasks. Retry waits use the Timer
/// injected through [`QubitSnowflakeGeneratorBuilder::timer`] and are
/// cancellation-safe.
#[must_use]
pub struct AsyncQubitSnowflakeGenerator {
    /// Asynchronous driver over the shared allocation core.
    inner: AsyncSnowflake<QubitSnowflakeLayout>,
}

impl AsyncQubitSnowflakeGenerator {
    /// Creates an asynchronous generator with Qubit defaults.
    ///
    /// # Parameters
    ///
    /// * `host` - Host identifier in `0..=511`.
    ///
    /// # Returns
    ///
    /// A configured asynchronous generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::HostOutOfRange`] when `host` does not fit the host
    /// field, [`IdError::ExpirationTimeOverflow`] when the lifetime boundary
    /// cannot be represented, or [`IdError::GeneratorExpired`] when the
    /// current wall time has reached that boundary.
    #[inline(always)]
    pub fn new(host: u64) -> Result<Self, IdError> {
        Self::builder(host).build_async()
    }

    /// Creates a configurable builder for an asynchronous Qubit generator.
    ///
    /// # Parameters
    ///
    /// * `host` - Host identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A builder shared with the synchronous Qubit generator.
    #[inline(always)]
    pub fn builder(host: u64) -> QubitSnowflakeGeneratorBuilder {
        QubitSnowflakeGeneratorBuilder::new(host)
    }

    /// Creates a public generator from a validated core and timer.
    ///
    /// # Parameters
    ///
    /// * `core` - Validated allocation core.
    /// * `timer` - Timer used for asynchronous retry waits.
    ///
    /// # Returns
    ///
    /// An asynchronous generator backed by `core` and `timer`.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<QubitSnowflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: AsyncSnowflake::new(core, timer),
        }
    }

    /// Returns the Qubit bit layout.
    ///
    /// # Returns
    ///
    /// The layout used to compose generated IDs.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::AsyncQubitSnowflakeGenerator;
    /// let generator =
    ///     AsyncQubitSnowflakeGenerator::new(7).expect("valid host");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub const fn layout(&self) -> &QubitSnowflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured timestamp origin.
    ///
    /// # Returns
    ///
    /// The timestamp origin represented by timestamp zero.
    #[must_use]
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    ///
    /// # Returns
    ///
    /// The first wall time that cannot be represented by this generator.
    #[must_use]
    #[inline(always)]
    pub const fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }

    /// Returns the maximum tolerated raw wall-clock rollback.
    ///
    /// # Returns
    ///
    /// The largest rollback duration that the generator may wait through.
    #[must_use]
    #[inline(always)]
    pub const fn max_clock_skew(&self) -> Duration {
        self.inner.core().max_clock_skew()
    }

    /// Generates the next Qubit Snowflake ID asynchronously.
    ///
    /// Concrete callers use this inherent method without allocating a boxed
    /// future. Use [`AsyncIdGenerator`] when object-safe dynamic dispatch is
    /// required.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future for the next generated ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when allocation or a retry wait fails.
    #[inline(always)]
    pub async fn generate_async(&self) -> Result<u64, IdError> {
        self.inner.generate().await
    }

    /// Generates an ID for an explicit wall time and sequence.
    ///
    /// This operation is stateless and provides no uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `time` - Wall time to encode.
    /// * `sequence` - Sequence to encode within the timestamp unit.
    ///
    /// # Returns
    ///
    /// The composed Qubit Snowflake ID.
    ///
    /// # Errors
    ///
    /// Returns [`IdError`] when the generator has expired or the values cannot
    /// be represented by the configured epoch and layout.
    #[inline(always)]
    pub fn generate_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<u64, IdError> {
        self.inner.core().generate_at(time, sequence)
    }
}

impl AsyncIdGenerator<u64> for AsyncQubitSnowflakeGenerator {
    /// Generates the next Qubit Snowflake ID asynchronously.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future for the next generated ID.
    ///
    /// # Errors
    ///
    /// The future resolves to [`IdError`] when allocation or a retry wait
    /// fails.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, u64> {
        Box::pin(AsyncQubitSnowflakeGenerator::generate_async(self))
    }
}

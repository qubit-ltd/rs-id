// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Asynchronous classic 41/10/12 Snowflake generator.

use std::sync::Arc;
use std::time::SystemTime;

use qubit_clock::Timer;

use super::internal::{
    AsyncSnowflake,
    SnowflakeCore,
};
use super::{
    SnowflakeGeneratorBuilder,
    SnowflakeLayout,
};
use crate::{
    AsyncIdGenerator,
    IdError,
    IdGenerationFuture,
};

/// Generates classic Snowflake IDs without blocking an asynchronous executor.
#[must_use]
pub struct AsyncSnowflakeGenerator {
    /// Asynchronous driver over the shared allocation core.
    inner: AsyncSnowflake<SnowflakeLayout>,
}

impl AsyncSnowflakeGenerator {
    /// Creates an asynchronous generator with the default Qubit epoch.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier in `0..=1023`.
    ///
    /// # Returns
    ///
    /// A configured asynchronous generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::NodeOutOfRange`] or
    /// [`IdError::ExpirationTimeOverflow`] for an invalid configuration, or
    /// [`IdError::GeneratorExpired`] when the current wall time has reached the
    /// exclusive expiration boundary.
    #[inline(always)]
    pub fn new(node_id: u64) -> Result<Self, IdError> {
        Self::builder(node_id).build_async()
    }

    /// Creates a builder shared with the synchronous generator.
    ///
    /// # Parameters
    ///
    /// * `node_id` - Node identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable classic Snowflake generator builder.
    #[inline(always)]
    pub fn builder(node_id: u64) -> SnowflakeGeneratorBuilder {
        SnowflakeGeneratorBuilder::new(node_id)
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
        core: SnowflakeCore<SnowflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: AsyncSnowflake::new(core, timer),
        }
    }

    /// Returns the configured classic Snowflake layout.
    ///
    /// # Returns
    ///
    /// The layout used to compose generated IDs.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::AsyncSnowflakeGenerator;
    /// let generator = AsyncSnowflakeGenerator::new(7).expect("valid node");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub const fn layout(&self) -> &SnowflakeLayout {
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

    /// Generates the next classic Snowflake ID asynchronously.
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
    /// Returns [`IdError::TimeBeforeEpoch`] when the wall clock precedes the
    /// epoch, [`IdError::GeneratorExpired`] at the lifetime boundary,
    /// [`IdError::ClockMovedBackwards`] after any wall-clock rollback, or
    /// [`IdError::WaitFailed`] when a retry wait cannot be registered or
    /// completed.
    #[inline(always)]
    pub async fn generate_async(&self) -> Result<u64, IdError> {
        self.inner.generate().await
    }
}

impl AsyncIdGenerator<u64> for AsyncSnowflakeGenerator {
    /// Generates the next classic Snowflake ID asynchronously.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future for the next generated ID.
    ///
    /// # Errors
    ///
    /// The future resolves to [`IdError::TimeBeforeEpoch`] when the wall clock
    /// precedes the epoch, [`IdError::GeneratorExpired`] at the lifetime
    /// boundary, [`IdError::ClockMovedBackwards`] after any wall-clock
    /// rollback, or [`IdError::WaitFailed`] when a retry wait cannot be
    /// registered or completed.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, u64> {
        Box::pin(AsyncSnowflakeGenerator::generate_async(self))
    }
}

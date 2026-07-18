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
    /// # Arguments
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
    /// [`IdError::ExpirationTimeOverflow`] for an invalid configuration.
    ///
    /// # Panics
    ///
    /// Panics when the current wall time has reached the exclusive expiration
    /// boundary.
    #[inline(always)]
    pub fn new(node_id: u64) -> Result<Self, IdError> {
        Self::builder(node_id).build_async()
    }

    /// Creates a builder shared with the synchronous generator.
    #[inline(always)]
    pub fn builder(node_id: u64) -> SnowflakeGeneratorBuilder {
        SnowflakeGeneratorBuilder::new(node_id)
    }

    /// Creates a public generator from a validated core and timer.
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
    #[inline(always)]
    pub const fn layout(&self) -> &SnowflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured timestamp origin.
    #[must_use]
    #[inline(always)]
    pub const fn epoch(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    #[must_use]
    #[inline(always)]
    pub const fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
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
    /// The future resolves to [`IdError`] when allocation or a retry wait
    /// fails.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, u64> {
        Box::pin(self.inner.generate())
    }
}

// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Asynchronous Sonyflake-style 63-bit ID generator.

use std::sync::Arc;
use std::time::SystemTime;

use qubit_clock::Timer;

use super::internal::{
    AsyncSnowflake,
    SnowflakeCore,
};
use super::{
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
};
use crate::{
    AsyncIdGenerator,
    IdError,
    IdGenerationFuture,
};

/// Generates Sonyflake-style IDs without blocking an asynchronous executor.
#[must_use]
pub struct AsyncSonyflakeGenerator {
    /// Asynchronous driver over the shared allocation core.
    inner: AsyncSnowflake<SonyflakeLayout>,
}

impl AsyncSonyflakeGenerator {
    /// Creates an asynchronous generator with the default layout.
    ///
    /// # Arguments
    ///
    /// * `machine_id` - Machine identifier in `0..=65535`.
    ///
    /// # Returns
    ///
    /// A configured asynchronous Sonyflake generator.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] when the default layout, start time, or lifetime
    /// is invalid.
    ///
    /// # Panics
    ///
    /// Panics when the current wall time has reached the exclusive expiration
    /// boundary.
    #[inline(always)]
    pub fn new(machine_id: u64) -> Result<Self, IdError> {
        Self::builder(machine_id).build_async()
    }

    /// Creates a builder shared with the synchronous generator.
    #[inline(always)]
    pub fn builder(machine_id: u64) -> SonyflakeGeneratorBuilder {
        SonyflakeGeneratorBuilder::new(machine_id)
    }

    /// Creates a public generator from a validated core and timer.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<SonyflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: AsyncSnowflake::new(core, timer),
        }
    }

    /// Returns the configured Sonyflake layout.
    #[inline(always)]
    pub const fn layout(&self) -> &SonyflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured elapsed-time origin.
    #[must_use]
    #[inline(always)]
    pub const fn start_time(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    #[must_use]
    #[inline(always)]
    pub const fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }
}

impl AsyncIdGenerator<u64> for AsyncSonyflakeGenerator {
    /// Generates the next Sonyflake-style ID asynchronously.
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

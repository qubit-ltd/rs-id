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
    /// # Parameters
    ///
    /// * `machine_id` - Machine identifier in `0..=65535`.
    ///
    /// # Returns
    ///
    /// A configured asynchronous Sonyflake generator.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::MachineIdOutOfRange`] when `machine_id` exceeds the
    /// default 16-bit field, [`IdError::ExpirationTimeOverflow`] when the
    /// lifetime boundary cannot be represented, [`IdError::StartTimeAhead`]
    /// when the default start time is later than the current wall clock, or
    /// [`IdError::GeneratorExpired`] when that clock has reached the boundary.
    #[inline(always)]
    pub fn new(machine_id: u64) -> Result<Self, IdError> {
        Self::builder(machine_id).build_async()
    }

    /// Creates a builder shared with the synchronous generator.
    ///
    /// # Parameters
    ///
    /// * `machine_id` - Machine identifier encoded by generated IDs.
    ///
    /// # Returns
    ///
    /// A configurable Sonyflake generator builder.
    #[inline(always)]
    pub fn builder(machine_id: u64) -> SonyflakeGeneratorBuilder {
        SonyflakeGeneratorBuilder::new(machine_id)
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
        core: SnowflakeCore<SonyflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
        Self {
            inner: AsyncSnowflake::new(core, timer),
        }
    }

    /// Returns the configured Sonyflake layout.
    ///
    /// # Returns
    ///
    /// The layout used to compose generated IDs.
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// #![deny(unused_must_use)]
    /// use qubit_id::AsyncSonyflakeGenerator;
    /// let generator = AsyncSonyflakeGenerator::new(7)
    ///     .expect("valid machine");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub const fn layout(&self) -> &SonyflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured elapsed-time origin.
    ///
    /// # Returns
    ///
    /// The wall time represented by elapsed time zero.
    #[must_use]
    #[inline(always)]
    pub const fn start_time(&self) -> SystemTime {
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

    /// Generates the next Sonyflake-style ID asynchronously.
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
    /// start time, [`IdError::GeneratorExpired`] at the lifetime boundary,
    /// [`IdError::ClockMovedBackwards`] after any wall-clock rollback, or
    /// [`IdError::WaitFailed`] when a retry wait cannot be registered or
    /// completed.
    #[inline(always)]
    pub async fn generate_async(&self) -> Result<u64, IdError> {
        self.inner.generate().await
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
    /// The future resolves to [`IdError::TimeBeforeEpoch`] when the wall clock
    /// precedes the start time, [`IdError::GeneratorExpired`] at the lifetime
    /// boundary, [`IdError::ClockMovedBackwards`] after any wall-clock
    /// rollback, or [`IdError::WaitFailed`] when a retry wait cannot be
    /// registered or completed.
    #[inline(always)]
    fn generate_async(&self) -> IdGenerationFuture<'_, u64> {
        Box::pin(AsyncSonyflakeGenerator::generate_async(self))
    }
}

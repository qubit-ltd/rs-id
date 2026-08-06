// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous Sonyflake-style 63-bit ID generator.

use std::sync::Arc;
use std::time::SystemTime;

use qubit_clock::Timer;

use super::internal::{BlockingSnowflake, SnowflakeCore};
use super::{SonyflakeGeneratorBuilder, SonyflakeLayout};
use crate::{
    AsyncIdGenerator, GenerationAttempt, IdError, IdGenerationFuture, IdGenerator, TryIdGenerator,
};

/// Default Sonyflake start time as Unix epoch milliseconds.
pub(super) const DEFAULT_START_MILLIS: u64 = 1_735_689_600_000;

/// Generates Sonyflake-style IDs with configurable time, sequence, and
/// machine fields.
///
/// The generator is thread-safe. One shared live instance never returns the
/// same ID twice, provided its machine identifier is exclusive within the ID
/// namespace. Retry waits use the timer injected through the builder.
#[derive(Clone)]
#[must_use]
pub struct SonyflakeGenerator {
    /// Blocking driver over the shared allocation core.
    inner: BlockingSnowflake<SonyflakeLayout>,
}

impl SonyflakeGenerator {
    /// Creates a generator with the default Sonyflake-style layout.
    ///
    /// # Parameters
    ///
    /// * `machine_id` - Machine identifier in `0..=65535`.
    ///
    /// # Returns
    ///
    /// A configured Sonyflake generator.
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
        Self::builder(machine_id).build()
    }

    /// Creates a configurable builder for a machine identifier.
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
    /// * `timer` - Timer adapted to blocking retry waits.
    ///
    /// # Returns
    ///
    /// A synchronous generator backed by `core` and `timer`.
    #[inline]
    pub(super) fn from_core(core: SnowflakeCore<SonyflakeLayout>, timer: Arc<dyn Timer>) -> Self {
        Self {
            inner: BlockingSnowflake::new(core, timer),
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
    /// use qubit_id::SonyflakeGenerator;
    /// let generator = SonyflakeGenerator::new(7).expect("valid machine");
    /// generator.layout();
    /// ```
    #[must_use = "use the returned layout reference"]
    #[inline(always)]
    pub fn layout(&self) -> &SonyflakeLayout {
        self.inner.core().layout()
    }

    /// Returns the configured elapsed-time origin.
    ///
    /// # Returns
    ///
    /// The wall time represented by elapsed time zero.
    #[must_use]
    #[inline(always)]
    pub fn start_time(&self) -> SystemTime {
        self.inner.core().epoch()
    }

    /// Returns the exclusive expiration boundary.
    ///
    /// # Returns
    ///
    /// The first wall time that cannot be represented by this generator.
    #[must_use]
    #[inline(always)]
    pub fn expires_at(&self) -> SystemTime {
        self.inner.core().expires_at()
    }

    /// Generates the next Sonyflake-style ID.
    ///
    /// This inherent method is convenient for concrete callers. Use
    /// [`IdGenerator`] when an object-safe dynamic-dispatch boundary is needed.
    ///
    /// # Returns
    ///
    /// The next generated Sonyflake-style ID.
    ///
    /// # Errors
    ///
    /// Returns the same errors as the [`IdGenerator::generate`] implementation.
    #[inline(always)]
    pub fn generate(&self) -> Result<u64, IdError> {
        self.inner.generate()
    }

    /// Attempts to generate an ID without sleeping or awaiting.
    ///
    /// # Returns
    ///
    /// A generated ID or the minimum delay before another attempt can make
    /// progress.
    ///
    /// # Errors
    ///
    /// Returns the non-retryable allocation errors described by
    /// [`IdGenerator::generate`].
    #[inline]
    pub fn try_generate(&self) -> Result<GenerationAttempt<u64>, IdError> {
        self.inner.try_generate()
    }

    /// Generates the next Sonyflake ID asynchronously.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future for the next generated ID.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`IdGenerator::generate`].
    pub async fn generate_async(&self) -> Result<u64, IdError> {
        self.inner.generate_async().await
    }
}

impl IdGenerator<u64> for SonyflakeGenerator {
    /// Generates the next Sonyflake-style ID.
    ///
    /// # Returns
    ///
    /// The next generated numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdError::TimeBeforeEpoch`] when the wall clock precedes the
    /// start time, [`IdError::GeneratorExpired`] at the lifetime boundary,
    /// [`IdError::ClockMovedBackwards`] after any wall-clock rollback, or
    /// [`IdError::WaitFailed`] when a retry wait cannot be registered or
    /// completed.
    #[inline(always)]
    fn generate(&self) -> Result<u64, IdError> {
        SonyflakeGenerator::generate(self)
    }
}

impl TryIdGenerator<u64> for SonyflakeGenerator {
    /// Attempts one non-blocking Sonyflake allocation.
    #[inline]
    fn try_generate(&self) -> Result<GenerationAttempt<u64>, IdError> {
        SonyflakeGenerator::try_generate(self)
    }
}

impl AsyncIdGenerator<u64> for SonyflakeGenerator {
    /// Generates a Sonyflake ID asynchronously.
    #[inline]
    fn generate_async(&self) -> IdGenerationFuture<'_, u64> {
        Box::pin(SonyflakeGenerator::generate_async(self))
    }
}

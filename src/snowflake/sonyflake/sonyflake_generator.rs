// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous Sonyflake-style 63-bit ID generator.

use std::sync::Arc;
use std::time::{
    Duration,
    SystemTime,
};

use qubit_clock::Timer;

use super::super::internal::{
    BlockingSnowflake,
    SnowflakeCore,
};
use super::{
    SonyflakeGeneratorBuilder,
    SonyflakeLayout,
};
use crate::{
    AsyncIdGenerator,
    BlockingIdGenerator,
    GenerationAttempt,
    Id,
    IdGenerationError,
    IdGenerationFuture,
    IdGenerator,
    TryIdGenerator,
};

/// Default Sonyflake epoch as Unix epoch milliseconds.
pub(super) const DEFAULT_EPOCH_MILLIS: u64 = 1_735_689_600_000;

/// Generates Sonyflake-style IDs with configurable time, sequence, and
/// machine fields.
///
/// The generator is thread-safe. One shared live instance never returns the
/// same ID twice, provided its machine identifier is exclusive within the ID
/// namespace. Retry waits use the timer injected through the builder. A
/// backwards clock movement within [`Self::max_clock_skew`] is retried after
/// waiting; a larger movement returns
/// [`IdGenerationError::ClockMovedBackwards`].
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
    /// Returns [`IdGenerationError::MachineIdOutOfRange`] when `machine_id`
    /// exceeds the default 16-bit field,
    /// [`IdGenerationError::ExpirationTimeOverflow`] when the
    /// lifetime boundary cannot be represented,
    /// [`IdGenerationError::EpochAhead`] when the default epoch is
    /// later than the current wall clock, or
    /// [`IdGenerationError::GeneratorExpired`] when that clock has reached the
    /// boundary.
    #[inline(always)]
    pub fn new(machine_id: u64) -> Result<Self, IdGenerationError> {
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
    /// A generator backed by `core` and `timer`.
    #[inline]
    pub(super) fn from_core(
        core: SnowflakeCore<SonyflakeLayout>,
        timer: Arc<dyn Timer>,
    ) -> Self {
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

    /// Returns the configured epoch.
    ///
    /// # Returns
    ///
    /// The wall time represented by elapsed time zero.
    #[must_use]
    #[inline(always)]
    pub fn epoch(&self) -> SystemTime {
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

    /// Returns the maximum tolerated backwards clock movement.
    ///
    /// # Returns
    ///
    /// Maximum tolerated raw wall-clock rollback.
    #[must_use]
    #[inline(always)]
    pub fn max_clock_skew(&self) -> Duration {
        self.inner.core().max_clock_skew()
    }

    /// Generates the next Sonyflake-style ID.
    ///
    /// This inherent method is convenient for concrete callers. Use
    /// [`BlockingIdGenerator`] when an object-safe dynamic-dispatch boundary is
    /// needed.
    ///
    /// # Returns
    ///
    /// The next generated Sonyflake-style ID.
    ///
    /// # Errors
    ///
    /// Returns the same errors as the [`BlockingIdGenerator::generate`]
    /// implementation.
    #[inline(always)]
    pub fn generate(&self) -> Result<Id, IdGenerationError> {
        self.inner.generate().map(Id::from)
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
    /// [`BlockingIdGenerator::generate`].
    #[inline]
    pub fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Id>, IdGenerationError> {
        self.inner
            .try_generate()
            .map(|attempt| attempt.map(Id::from))
    }

    /// Generates the next Sonyflake ID asynchronously.
    ///
    /// # Returns
    ///
    /// A cancellation-safe future for the next generated ID.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`BlockingIdGenerator::generate`].
    pub async fn generate_async(&self) -> Result<Id, IdGenerationError> {
        self.inner.generate_async().await.map(Id::from)
    }

    /// Composes an ID for an explicit time and sequence.
    ///
    /// This operation is stateless and provides no uniqueness guarantee.
    ///
    /// # Parameters
    ///
    /// * `time` - Wall time to encode.
    /// * `sequence` - Sequence to encode within that elapsed-time unit.
    ///
    /// # Returns
    ///
    /// A Sonyflake ID containing the specified elapsed time and sequence.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] if `time` is before the
    /// configured epoch, [`IdGenerationError::GeneratorExpired`] if `time`
    /// has reached the exclusive expiration boundary, or
    /// [`IdGenerationError::SequenceOverflow`] when `sequence` does not fit
    /// the layout.
    #[inline(always)]
    pub fn compose_at(
        &self,
        time: SystemTime,
        sequence: u64,
    ) -> Result<Id, IdGenerationError> {
        self.inner.core().compose_at(time, sequence).map(Id::from)
    }
}

impl IdGenerator for SonyflakeGenerator {
    type Output = Id;
    type Error = IdGenerationError;
}

impl BlockingIdGenerator for SonyflakeGenerator {
    /// Generates the next Sonyflake-style ID.
    ///
    /// # Returns
    ///
    /// The next generated numeric identifier.
    ///
    /// # Errors
    ///
    /// Returns [`IdGenerationError::TimeBeforeEpoch`] when the wall clock
    /// precedes the epoch, [`IdGenerationError::GeneratorExpired`] at
    /// the lifetime boundary, [`IdGenerationError::ClockMovedBackwards`] when
    /// a wall-clock rollback exceeds the configured tolerance, or
    /// [`IdGenerationError::WaitFailed`] when a retry wait cannot be
    /// registered or completed.
    #[inline(always)]
    fn generate(&self) -> Result<Self::Output, Self::Error> {
        SonyflakeGenerator::generate(self)
    }
}

impl TryIdGenerator for SonyflakeGenerator {
    /// Attempts one non-blocking Sonyflake allocation.
    #[inline]
    fn try_generate(
        &self,
    ) -> Result<GenerationAttempt<Self::Output>, Self::Error> {
        SonyflakeGenerator::try_generate(self)
    }
}

impl AsyncIdGenerator for SonyflakeGenerator {
    /// Generates a Sonyflake ID asynchronously.
    #[inline]
    fn generate_async(
        &self,
    ) -> IdGenerationFuture<'_, Self::Output, Self::Error> {
        Box::pin(SonyflakeGenerator::generate_async(self))
    }
}
